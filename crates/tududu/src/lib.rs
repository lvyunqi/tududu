//! 图嘟嘟服务端。
//!
//! 单二进制，通过子命令承担不同角色：HTTP API 与后台任务。

pub mod admin;
pub mod admin_auth;
pub mod auth;
pub mod billing;
pub mod cli;
pub mod config;
pub mod db;
pub mod env_file;
pub mod error;
pub mod http;
pub mod job;
pub mod payment;
pub mod plaza;
pub mod provider;
pub mod ratelimit;
pub mod session;
pub mod settings;
pub mod sign;
pub mod storage;
pub mod tasks;
pub mod telemetry;
