//! 图嘟嘟服务端。
//!
//! 单二进制，通过子命令承担不同角色：HTTP API 与后台任务。

pub mod auth;
pub mod cli;
pub mod config;
pub mod db;
pub mod error;
pub mod http;
pub mod telemetry;
