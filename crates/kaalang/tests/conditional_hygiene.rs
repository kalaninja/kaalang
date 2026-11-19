//! A consumer can supply its own `core` while using the conditional flow.

#![no_std]

extern crate self as core;

#[path = "flow/method/conditional_parameters.rs"]
mod conditional_parameters;
