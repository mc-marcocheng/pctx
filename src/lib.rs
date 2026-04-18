//! pctx - Generate LLM-ready context from your codebase
//!
//! This library provides functionality to scan codebases and generate
//! formatted context suitable for Large Language Models.

pub mod cli;
pub mod config;
pub mod content;
pub mod error;
pub mod exit_codes;
pub mod filter;
pub mod output;
pub mod scanner;
pub mod stats;