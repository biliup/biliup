//! PCRE2 10.49's regular-expression algorithms implemented in Rust.
//!
//! The raw interface preserves the 8-, 16-, and 32-bit PCRE2 C interfaces.
//! Raw pointers, callbacks, contexts, and their lifetimes follow the same
//! contracts as the corresponding PCRE2 functions.

// PCRE2 10.49 Rust translation.
// Basic library: Copyright (c) 1997-2007 University of Cambridge;
// Copyright (c) 2007-2024 Philip Hazel. All rights reserved.
// PCRE2 JIT: Copyright (c) 2010-2024 Zoltan Herczeg. All rights reserved.
// SLJIT: Copyright (c) 2009-2024 Zoltan Herczeg. All rights reserved.
// Other contributions remain copyright their original authors or employers.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// * Redistributions of source code must retain the above copyright notices,
//   this list of conditions and the following disclaimer.
// * Redistributions in binary form must reproduce the above copyright
//   notices, this list of conditions and the following disclaimer in the
//   documentation and/or other materials provided with the distribution.
// * Neither the name of the University of Cambridge nor the names of any
//   contributors may be used to endorse or promote products derived from this
//   software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
//
// PCRE2 exception: The second condition does not apply all the way down a
// chain of software. Binary package A including PCRE2 must respect it, but
// package B including A is not subject to it unless B independently uses PCRE2.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(dead_code, unused_mut, unused_assignments, unused_imports)]
#![allow(unused_parens, unused_variables)]
// Width modules declare the same C-compatible pointer interfaces using their
// private definitions of the original structs; public bindings keep them opaque.
#![allow(clashing_extern_declarations, unused_unsafe)]

mod bindings;
pub use bindings::*;
mod jit_support;
mod width8;
mod width16;
mod width32;
mod posix;
