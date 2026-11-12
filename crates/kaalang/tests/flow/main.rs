//! Flows: each directory covers one language feature with one flow per file,
//! named for the flow it declares, and the diagram it draws beside it. Its
//! `compile_fail` directory documents the programs that feature rejects.

mod action;
mod call;
mod capture;
mod choice;
mod cycle;
mod empty_flow;
mod end;
mod method;
mod question;
mod stage;
mod syntax;
mod wire;
