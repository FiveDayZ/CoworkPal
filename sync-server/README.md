# CoworkPal Sync Server

用于保存 CoworkPal 用户数据快照的轻量 Docker 服务。服务不依赖数据库，每次上传都会保留一个版本，默认每位用户保留最近 30 个版本。

## 部署

1. 复制 `.env.example` 为 `.env`，配置一个至少 32 个字符的管理员令牌。`COWORKPAL_USERS_JSON` 可保留 `{}`；需要兼容旧客户端时，也可继续预配置用户令牌。
2. 执行 `docker compose up -d --build`。主机端口默认是 8080；端口冲突时可在 `.env` 设置 `COWORKPAL_SYNC_PORT`。
3. 使用 Caddy、Nginx 或服务器现有网关把 HTTPS 域名反向代理到 `127.0.0.1:8080`。
4. 用户在 CoworkPal 设置页填写 HTTPS 地址和用户名，点击“申请令牌”。管理员在看板批准后，客户端会自动领取并保存令牌，同时上传首次完整备份。

示例反向代理目标为 `http://127.0.0.1:8080`。除本机测试外，客户端会拒绝明文 HTTP，避免访问令牌和笔记内容在网络中泄露。

最小 Caddy 配置示例：

```caddy
sync.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

部署后可执行 `curl https://sync.example.com/health` 检查服务，预期返回 `{"status":"ok"}`。

## 管理看板

浏览器打开同步服务根地址（例如 `https://sync.example.com/`），输入 `COWORKPAL_ADMIN_TOKEN` 即可处理客户端令牌申请并查看全部用户。点击“发放令牌”后，令牌只会返回给持有本机申请凭据的客户端，不会显示在管理看板中。看板顶部汇总用户数、备份数、累计在线、笔记、日报、体检和成就；点击用户行可按“备份信息、工坊状态、云端数据、应用设置、硬件配置、模块等级”查看详情，其中硬件配置包含备份时采集的 CPU、显卡、内存、主板、磁盘等静态信息。尚未上传备份的用户也会显示，旧版本备份需要由新版客户端重新上传后才会出现硬件配置。

管理员令牌仅保存在当前浏览器标签页的 `sessionStorage` 中，关闭标签页后自动清除。请勿将管理员令牌配置为任一用户令牌，也不要绕过 HTTPS 暴露服务。

## 数据与备份

- 数据保存在 Docker 卷 `coworkpal-sync-data`。
- 管理员发放的用户凭据和申请状态保存在卷内的 `auth-state.json`；客户端完成首次备份后，服务端会移除申请记录中的明文令牌，只保留令牌摘要。
- 每个用户目录由用户名的 SHA-256 摘要命名，日志不会输出令牌或快照内容。
- 备份服务器时请同时备份该 Docker 卷。
- 修改 `MAX_VERSIONS` 可以调整每位用户保留的历史版本数。

服务接口：`GET /health`、`POST /v1/token-requests`、`GET /v1/token-requests/:id`、`GET /v1/snapshot`、`PUT /v1/snapshot`、`GET /v1/admin/snapshots`、`POST /v1/admin/token-requests/:id/approve`、`POST /v1/admin/token-requests/:id/reject`。快照接口使用用户 Bearer Token 鉴权，管理接口使用管理员 Bearer Token 鉴权。
