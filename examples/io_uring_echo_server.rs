use std::collections::VecDeque;
use std::net::TcpListener;
use std::os::fd::RawFd;
use std::{io, os::fd::AsRawFd};

use io_uring::cqueue::{self, more};
use io_uring::{opcode, squeue, types, IoUring, SubmissionQueue, Submitter};
use slab::Slab;

#[derive(Clone, Debug)]
struct PollToken {
    fd: RawFd,
}

#[derive(Clone, Debug)]
struct ReadToken {
    fd: RawFd,
    buf_index: usize,
}

#[derive(Clone, Debug)]
struct WriteToken {
    fd: RawFd,
    buf_index: usize,
    offset: usize,
    len: usize,
}

#[derive(Clone, Debug)]
enum Token {
    Accept,
    Poll(PollToken),
    Read(ReadToken),
    Write(WriteToken),
}

struct State<'s> {
    sq: SubmissionQueue<'s>,
    buf_pool: Vec<usize>,
    buf_alloc: Slab<Box<[u8]>>,
    token_alloc: Slab<Token>,
    backlog: VecDeque<squeue::Entry>,
}

impl<'s> State<'s> {
    pub fn new(sq: SubmissionQueue<'s>) -> Self {
        Self {
            sq,
            buf_pool: Vec::with_capacity(64),
            buf_alloc: Slab::with_capacity(64),
            token_alloc: Slab::with_capacity(64),
            backlog: VecDeque::new(),
        }
    }

    pub fn push_entry(&mut self, entry: squeue::Entry) {
        unsafe {
            if self.sq.push(&entry).is_err() {
                self.backlog.push_back(entry);
            }
        }
    }
}

fn main() -> io::Result<()> {
    let mut ring = IoUring::new(256)?;
    let listener = TcpListener::bind(("127.0.0.1", 3456))?;

    println!("listening on {}", listener.local_addr()?);

    let (submitter, sq, mut cq) = ring.split();
    let mut state = State::new(sq);

    let accept = opcode::AcceptMulti::new(types::Fd(listener.as_raw_fd()))
        .build()
        .user_data(state.token_alloc.insert(Token::Accept) as _);

    unsafe { state.sq.push(&accept).expect("SQ was full") };
    state.sq.sync();

    loop {
        match submitter.submit_and_wait(1) {
            Ok(_) => (),
            Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => (),
            Err(err) => return Err(err),
        }

        drain_backlog(&mut state, &submitter)?;
        cq.sync();

        for cqe in &mut cq {
            if cqe.result() < 0 {
                eprintln!(
                    "token {:?} error: {:?}",
                    state.token_alloc[cqe.user_data() as usize],
                    io::Error::from_raw_os_error(-cqe.result())
                );
                continue;
            }

            match state.token_alloc[cqe.user_data() as usize].clone() {
                Token::Accept => accept_conn(&mut state, cqe, types::Fd(listener.as_raw_fd()))?,
                Token::Poll(token) => poll_conn(&mut state, cqe, token)?,
                Token::Read(token) => read_conn(&mut state, cqe, token)?,
                Token::Write(token) => write_conn(&mut state, cqe, token)?,
            }
        }

        state.sq.sync();
    }
}

fn drain_backlog(state: &mut State<'_>, submitter: &Submitter) -> io::Result<()> {
    loop {
        if state.sq.is_full() {
            match submitter.submit() {
                Ok(_) => (),
                Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => return Ok(()),
                Err(err) => return Err(err),
            }
        }
        state.sq.sync();

        match state.backlog.pop_front() {
            Some(sqe) => unsafe {
                let _ = state.sq.push(&sqe);
            },
            None => return Ok(()),
        }
    }
}

fn accept_conn(
    state: &mut State<'_>,
    cqe: cqueue::Entry,
    listener_fd: types::Fd,
) -> io::Result<()> {
    if !more(cqe.flags()) {
        let accept = opcode::AcceptMulti::new(listener_fd)
            .build()
            .user_data(state.token_alloc.insert(Token::Accept) as _);

        unsafe { state.sq.push(&accept).expect("SQ was full") };
    }

    let fd = cqe.result();
    let poll_token = state.token_alloc.insert(Token::Poll(PollToken { fd }));

    let poll_entry = opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
        .build()
        .user_data(poll_token as _);

    state.push_entry(poll_entry);

    Ok(())
}

fn poll_conn(state: &mut State<'_>, cqe: cqueue::Entry, token: PollToken) -> io::Result<()> {
    let (buf_index, buf) = match state.buf_pool.pop() {
        Some(buf_index) => (buf_index, &mut state.buf_alloc[buf_index]),
        None => {
            let buf = vec![0u8; 2048].into_boxed_slice();
            let buf_entry = state.buf_alloc.vacant_entry();
            let buf_index = buf_entry.key();
            (buf_index, buf_entry.insert(buf))
        }
    };

    let token_index = cqe.user_data() as usize;
    state.token_alloc[token_index] = Token::Read(ReadToken {
        fd: token.fd,
        buf_index,
    });

    let read_entry = opcode::Recv::new(types::Fd(token.fd), buf.as_mut_ptr(), buf.len() as _)
        .build()
        .user_data(token_index as _);

    state.push_entry(read_entry);

    Ok(())
}

fn read_conn(state: &mut State<'_>, cqe: cqueue::Entry, token: ReadToken) -> io::Result<()> {
    // connection closed
    let ret = cqe.result();
    let token_index = cqe.user_data() as usize;

    if ret == 0 {
        state.buf_pool.push(token.buf_index);
        state.token_alloc.remove(token_index);

        unsafe {
            libc::close(token.fd);
        }
    } else {
        let len = ret as usize;
        let buf = &state.buf_alloc[token.buf_index];

        state.token_alloc[token_index] = Token::Write(WriteToken {
            fd: token.fd,
            buf_index: token.buf_index,
            offset: 0,
            len,
        });

        let write_entry = opcode::Send::new(types::Fd(token.fd), buf.as_ptr(), len as _)
            .build()
            .user_data(token_index as _);

        state.push_entry(write_entry);
    }

    Ok(())
}

fn write_conn(state: &mut State<'_>, cqe: cqueue::Entry, token: WriteToken) -> io::Result<()> {
    let write_len = cqe.result() as usize;
    let token_index = cqe.user_data() as usize;

    let entry = if token.offset + write_len >= token.len {
        state.buf_pool.push(token.buf_index);
        state.token_alloc[token_index] = Token::Poll(PollToken { fd: token.fd });

        opcode::PollAdd::new(types::Fd(token.fd), libc::POLLIN as _)
            .build()
            .user_data(token_index as _)
    } else {
        // write was incomplete. Requeue with remaining to be written
        let offset = token.offset + write_len;
        let len = token.len - offset;

        let buf = &state.buf_alloc[token.buf_index][offset..];

        state.token_alloc[token_index] = Token::Write(WriteToken {
            fd: token.fd,
            buf_index: token.buf_index,
            offset,
            len,
        });

        opcode::Write::new(types::Fd(token.fd), buf.as_ptr(), len as _)
            .build()
            .user_data(token_index as _)
    };

    state.push_entry(entry);

    Ok(())
}
