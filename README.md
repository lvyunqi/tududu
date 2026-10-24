# 图嘟嘟

手机端 AI 修图应用的服务端与配套工程。用户上传照片，用自然语言描述想要的修改，由图像模型完成编辑；
结果保存在本机磁盘上，可整理进相册，也可以连同提示词发布到广场供他人套用。

后端用 Rust 编写，编译为单个二进制文件；数据库为 PostgreSQL；移动端用 Tauri 2 打包成 Android 应用。

## 目录

```
crates/tududu/     服务端（库 + 二进制）
.github/workflows/ 持续集成
```

## 本地开发

需要 Rust 1.98 或更新版本。

```bash
cargo build
cargo test
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

启动 HTTP 服务（会在启动时自动执行数据库迁移）：

```bash
cargo run -- serve
curl http://127.0.0.1:8080/api/v1/health
```

## 接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/healthz` | 存活探针，不访问数据库 |
| GET | `/api/v1/health` | 健康检查，包含数据库连通性 |
| POST | `/api/v1/auth/register` | 注册，手机号或邮箱加密码 |
| POST | `/api/v1/auth/login` | 登录，返回访问令牌与刷新令牌 |
| POST | `/api/v1/auth/refresh` | 刷新访问令牌，同时轮换刷新令牌 |
| POST | `/api/v1/auth/logout` | 登出当前设备 |
| GET | `/api/v1/me` | 当前登录用户资料 |

除注册、登录、刷新外，其余接口需要 `Authorization: Bearer <access_token>`。

## 配置

启动参数目前全部来自环境变量：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `TUDUDU_LISTEN` | `0.0.0.0:8080` | HTTP 监听地址 |
| `TUDUDU_DATA_DIR` | `./data` | 本地数据目录，存放上传与生成的图片 |
| `TUDUDU_DATABASE_URL` | 无，必填 | PostgreSQL 连接串 |
| `TUDUDU_DB_MAX_CONNECTIONS` | `10` | 连接池上限 |
| `TUDUDU_JWT_SECRET` | 无，必填 | 访问令牌签名密钥，至少 32 位 |
| `TUDUDU_ACCESS_TTL` | `900` | 访问令牌有效期，单位秒 |
| `TUDUDU_REFRESH_TTL` | `2592000` | 刷新令牌有效期，单位秒 |
| `TUDUDU_LOG` | `info` | 日志过滤规则，语法同 `RUST_LOG` |

## 构建

CI 在 Linux 上编译静态二进制并作为构建产物上传：

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

## 开发约定

- 改动走分支和 Pull Request，CI 通过后再合并到 `main`。
- 提交前在本地跑通 `fmt`、`clippy`、`test`，不要依赖 CI 发现问题。
- 金额一律用整数表示最小单位，不使用浮点数。
