# 图嘟嘟

手机端 AI 修图应用的服务端与配套工程。用户上传照片，用自然语言描述想要的修改，由图像模型完成编辑；
结果保存在本机磁盘上，可整理进相册，也可以连同提示词发布到广场供他人套用。

后端用 Rust 编写，编译为单个二进制文件；数据库为 PostgreSQL；移动端用 Tauri 2 打包成 Android 应用。

## 目录

```
crates/tududu/     服务端（库 + 二进制）
web/               用户端（React + TypeScript + Vite）
admin/             管理后台（React + TypeScript + Vite）
.github/workflows/ 持续集成
```

## 前端

```bash
cd web
pnpm install
pnpm dev      # 本地开发，/api 会转发到 127.0.0.1:8080
              # 页面路由用 hash：创作 / 广场 / 相册 / 我的 / 充值
pnpm typecheck
pnpm build
```

管理后台单独一个工程，开发时另开一个终端：

```bash
cd admin
pnpm install
pnpm dev      # 5174 端口，/api 同样转发到 8080
```

登录时填服务端配置的 `TUDUDU_ADMIN_TOKEN`。

## 本地开发

需要 Rust 1.98 或更新版本。

```bash
cargo build
cargo test
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

计费与支付的集成测试需要一个 PostgreSQL：

```bash
TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/tududu_test \
  cargo test --test billing_flow -- --test-threads=1
```

前端构建产物会被编译进二进制。想让二进制同时服务网页，先构建前端再编译后端：

```bash
cd web && pnpm build
cd ../admin && pnpm build
cd .. && cargo build --release
```

启动后 `/` 是用户端，`/admin` 是管理后台，`/api/v1` 是接口。若没有构建前端，
访问页面会提示界面未构建，接口照常可用。

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
| POST | `/api/v1/uploads` | 上传图片，multipart 字段名 `file`，仅 PNG/JPEG/WebP，单张不超过 20MB |
| GET | `/api/v1/assets` | 我的图片列表，游标分页（`before`、`limit`） |
| GET | `/api/v1/assets/{id}` | 图片详情 |
| GET | `/api/v1/assets/{id}/file` | 取图片原始字节，加 `?size=thumb` 取缩略图 |
| DELETE | `/api/v1/assets/{id}` | 移入回收站，默认保留 30 天后清理 |
| GET | `/api/v1/storage/quota` | 当前占用与上限 |
| POST | `/api/v1/jobs` | 提交修图任务，立即返回任务号，上游调用在后台执行 |
| GET | `/api/v1/jobs/{id}` | 查询任务状态与结果图 |
| GET/POST | `/api/v1/sessions` | 会话列表与新建 |
| GET/PATCH/DELETE | `/api/v1/sessions/{id}` | 会话详情（含消息）、改名、删除 |
| POST | `/api/v1/sessions/{id}/messages` | 提交一轮修改，返回任务号 |
| GET | `/api/v1/prompt-categories` | 广场分类 |
| GET/POST | `/api/v1/prompts` | 广场列表与发布 |
| GET/DELETE | `/api/v1/prompts/{code}` | 作品详情与下架（仅作者） |
| POST | `/api/v1/prompts/{code}/like` | 点赞或取消 |
| GET | `/api/v1/my/prompts` | 我发布的作品 |

发布时校验标题与提示词长度，封面图必须是自己的图片。广场列表只展示已发布的作品，按发布时间倒序。

标题或提示词命中敏感词时，作品会进入待审核状态而不是直接上架；管理端通过后才公开。

| GET | `/api/v1/billing/wallet` | 钱包余额 |
| GET | `/api/v1/billing/ledger` | 流水，游标分页 |
| GET | `/api/v1/billing/config` | 计费单位名称与换算，公开 |
| GET | `/api/v1/billing/plans` | 充值套餐 |
| POST | `/api/v1/billing/orders` | 下单，返回易支付跳转地址 |
| GET | `/api/v1/billing/orders/{order_no}` | 订单状态，前端轮询用 |
| GET/POST | `/pay/epay/notify` | 易支付异步通知，返回纯文本 `success` |
| GET | `/pay/epay/return` | 支付完成后的回跳落地页 |

支付走易支付标准协议：签名是参数按名排序后拼 `k=v&...` 再拼商户密钥取 MD5。回调会校验签名与金额，同一订单重复回调只入账一次。

任务提交时先按模型、尺寸、质量查价格并预扣，失败自动退回。余额不足直接返回 40201。

任务状态：`0` 排队、`1` 执行中、`2` 成功、`3` 失败、`4` 已取消。失败会自动换渠道重试，最多 3 次。

会话是带图的连续对话：每轮写一条用户消息和一条助手消息，任务跑完由执行器把结果图回写到助手消息上。

提交一轮修改时如果没带 `input_asset_id`，服务端会自动拿本会话最近一张结果图当输入，
所以「再自然一点」这类省略主语的指令也能接着上一版改。要基于新图重来就显式传 `input_asset_id`。

限流是进程内计数，触发时返回 `10004`：登录同一账号五分钟十次、提交任务一小时三十个、上传一小时六十张。
状态在内存里，重启清零，多实例各自计数。

图片落在 `TUDUDU_DATA_DIR` 下，库里只存相对路径。同一用户重复上传相同内容会直接复用已有记录，
也不会重复占用配额。超过 `TUDUDU_QUOTA_BYTES` 时上传返回 50002。

管理接口用管理员账号登录，拿到令牌后放 `Authorization: Bearer <token>`。
过渡期仍支持请求头 `x-admin-token`，值为 `TUDUDU_ADMIN_TOKEN`。

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/api/v1/admin/auth/login` | 账号密码登录，返回 8 小时有效的令牌 |


| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/v1/admin/providers` | 渠道商列表 |
| POST | `/api/v1/admin/providers` | 新增渠道商 |
| PATCH | `/api/v1/admin/providers/{id}` | 修改渠道商 |
| DELETE | `/api/v1/admin/providers/{id}` | 删除渠道商 |
| GET | `/api/v1/admin/providers/{id}/channels` | 该渠道商的密钥列表 |
| POST | `/api/v1/admin/providers/{id}/channels` | 添加密钥 |
| GET | `/api/v1/admin/providers/{id}/models` | 已收录的模型 |
| GET/POST | `/api/v1/admin/models` | 全部模型与手工添加 |
| PATCH | `/api/v1/admin/models/{id}` | 启用或停用模型 |
| GET | `/api/v1/models` | 用户端可选的模型（已启用且渠道商启用） |
| POST | `/api/v1/admin/providers/{id}/models/refresh` | 调用上游 `/models` 在线拉取并入库 |
| GET/PUT | `/api/v1/admin/billing/prices` | 查询与批量设置价格（模型 × 尺寸 × 质量） |
| GET/PUT | `/api/v1/admin/billing/config` | 计费单位名称与每元换算数量 |
| POST | `/api/v1/admin/users/{id}/credits` | 手工增减余额，写流水 |
| GET/POST | `/api/v1/admin/recharge-plans` | 充值套餐的查询与新增 |
| GET/POST | `/api/v1/admin/pay-channels` | 易支付通道的查询与新增（商户密钥加密入库） |
| GET | `/api/v1/admin/overview` | 看板数据：用户、充值、任务、作品数 |
| GET | `/api/v1/admin/reconcile` | 手动跑一轮对账：关闭过期订单、核对账本 |
| GET | `/api/v1/admin/settings` | 运行时配置列表，带当前值与是否来自数据库 |
| PUT | `/api/v1/admin/settings/{key}` | 改一项运行时配置，保存后立刻生效 |
| GET | `/api/v1/admin/users` | 用户列表，带余额 |
| PATCH | `/api/v1/admin/users/{id}/status` | 改用户状态（正常 / 禁言 / 封禁） |
| GET | `/api/v1/admin/pay-orders` | 充值订单列表 |
| GET | `/api/v1/admin/jobs` | 任务列表，可按状态筛 |
| GET | `/api/v1/admin/jobs/{id}` | 任务详情，含每次上游调用记录 |
| GET | `/api/v1/admin/prompts` | 广场作品列表，可按状态筛（1 待审 / 2 已发布 / 3 已下架） |
| GET/POST | `/api/v1/admin/sensitive-words` | 敏感词表查询与新增 |
| DELETE | `/api/v1/admin/sensitive-words/{id}` | 删除词条 |
| POST | `/api/v1/admin/prompts/{code}/status` | 下架或恢复作品 |

管理端的写操作会写一条 `audit_logs` 留痕。

价格表里 `size`、`quality` 留空表示通配，匹配时优先用更具体的规则。

渠道密钥入库前用 AES-256-GCM 加密，接口只返回掩码。

除注册、登录、刷新外，其余接口需要 `Authorization: Bearer <access_token>`。

## 配置

启动参数从环境变量读取，也可以写进 `.env` 文件。服务启动时会自动加载当前目录的 `.env`，
真实环境变量优先级更高，不会被文件覆盖；用 `TUDUDU_ENV_FILE` 可以指定别的路径。

仓库里有 `.env.example`，复制成 `.env` 按需改即可：

```bash
cp .env.example .env
```


| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `TUDUDU_LISTEN` | `0.0.0.0:8080` | HTTP 监听地址 |
| `TUDUDU_DATA_DIR` | `./data` | 本地数据目录，图片按 `u/{用户短编号}/{年}/{月}/` 存放 |
| `TUDUDU_DATABASE_URL` | 无，必填 | PostgreSQL 连接串 |
| `TUDUDU_DB_MAX_CONNECTIONS` | `10` | 连接池上限 |
| `TUDUDU_JWT_SECRET` | 无，必填 | 访问令牌签名密钥，至少 32 位 |
| `TUDUDU_ACCESS_TTL` | `900` | 访问令牌有效期，单位秒 |
| `TUDUDU_REFRESH_TTL` | `2592000` | 刷新令牌有效期，单位秒 |
| `TUDUDU_ADMIN_USERNAME` | 空 | 初始管理员账号，只在库里没有管理员时创建 |
| `TUDUDU_ADMIN_PASSWORD` | 空 | 初始管理员密码，同上 |
| `TUDUDU_ADMIN_TOKEN` | 空 | 旧的单一令牌，保留作为过渡，账号体系就绪后可以不配 |
| `TUDUDU_TRASH_DAYS` | `30` | 回收站保留天数，到期后文件会被真正删掉 |
| `TUDUDU_QUOTA_BYTES` | `2147483648` | 单用户存储上限（字节），默认 2GB |
| `TUDUDU_CORS_ORIGINS` | `tauri://localhost` 等 | 允许跨域的来源，逗号分隔；默认放行 App 的 WebView |
| `TUDUDU_PUBLIC_BASE_URL` | 空 | 对外访问地址，用于拼接支付回调地址 |
| `TUDUDU_LOG` | `info` | 日志过滤规则，语法同 `RUST_LOG` |
| `TUDUDU_ENV_FILE` | `./.env` | 环境变量文件路径 |

### 在线可改的配置

下面几项存在数据库里，后台「配置」页改完立即生效，不用重启：

| 配置项 | 说明 |
| --- | --- |
| `access_ttl_secs` | 访问令牌有效期 |
| `refresh_ttl_secs` | 刷新令牌有效期 |
| `user_quota_bytes` | 单用户存储上限 |
| `trash_retention_days` | 回收站保留天数 |
| `public_base_url` | 对外访问地址 |

数据库里的值优先于环境变量，所以在后台改过之后，机器上的 `.env` 就不再影响这几项。
监听地址、数据库连接串、签名密钥、跨域名单属于启动期参数，只能通过环境变量配置。

## 构建

CI 在 Linux 上编译静态二进制并作为构建产物上传：

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

### Android 签名

安卓包的签名密钥库存在仓库 Secrets 里，构建时还原成文件再签名。

| Secret | 用途 |
| --- | --- |
| `ANDROID_KEYSTORE_BASE64` | 密钥库文件，base64 编码 |
| `ANDROID_KEYSTORE_PASSWORD` | 密钥库口令 |
| `ANDROID_KEY_PASSWORD` | 密钥口令 |
| `ANDROID_KEY_ALIAS` | 密钥别名 |
| `ANDROID_KEYSTORE_SHA256` | 证书 SHA-256 指纹，用于核对签名是否一致 |

当前证书指纹：`D45BED1BB8A1BBD09D4B191B4B8E1A69881AFFCC650A324480441C47B000FEFC`。

这个密钥库是最终发布用的，丢了就没法给已安装的用户推更新。改动它之前先想清楚。

## 开发约定

- 改动走分支和 Pull Request，CI 通过后再合并到 `main`。
- 提交前在本地跑通 `fmt`、`clippy`、`test`，不要依赖 CI 发现问题。
- 金额一律用整数表示最小单位，不使用浮点数。
