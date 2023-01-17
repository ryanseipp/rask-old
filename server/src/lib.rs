// Copyright 2022 Ryan Seipp
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![deny(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unused_imports,
    // dead_code
)]
// temporary
#![allow(dead_code)]
// Disallow warnings in examples.
#![doc(test(attr(deny(warnings))))]

//! http is a low-level HTTP implementation intended for personal learning purposes.
//!
//! ## Examples
//!
//! Examples can be found in the `examples` directory of the source code, or [on GitHub].

mod buffer;
pub mod listener;
pub mod sessions;
pub mod worker;
