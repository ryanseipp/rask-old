import http from 'k6/http';

export const options = {
  stages: [
    {duration: '5s', target: 100},
    {duration: '5s', target: 200},
    {duration: '5s', target: 300},
    // {duration: '10s', target: 400},
    // {duration: '30s', target: 500},
    // {duration: '10s', target: 600},
    // {duration: '10s', target: 700},
    // {duration: '10s', target: 800},
    // {duration: '10s', target: 900},
    // {duration: '30s', target: 1000},
    // {duration: '10s', target: 900},
    // {duration: '10s', target: 800},
    // {duration: '10s', target: 700},
    // {duration: '10s', target: 600},
    // {duration: '10s', target: 500},
    // {duration: '10s', target: 400},
    {duration: '35s', target: 300},
    {duration: '5s', target: 200},
    {duration: '5s', target: 100},
  ]
}

export default function() {
  http.get('http://127.0.0.1:80');
}
