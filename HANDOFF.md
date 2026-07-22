# Handoff: Solana 链上充值功能（给 Codex）

> 状态更新：2026-07-22。devnet 合约已部署并初始化；Solana 版 sub2api 与 watcher 已部署到 Netcup 的 Eyami 环境并稳定运行。
> 下一步只剩用已登录的测试用户和 devnet 钱包做真实充值、自动入账与提款验证。

## 1. 功能概览

为 sub2api（Go 后端 + Vue 前端）新增 Solana 链上充值：

```
前端充值页（连 Solana 钱包）→ 调 Anchor 合约 deposit_sol/deposit_usdc
→ 资金进 Vault PDA，合约 emit DepositEvent{user_id, token, amount}
→ solana-watcher（Rust）监听事件（finalized）→ 折算 USD
→ 调 Admin API POST /api/v1/admin/redeem-codes/create-and-redeem 加余额
管理员用 scripts/withdraw.js 从 Vault 提款
```

- 完整接入文档：`docs/SOLANA_DEPOSIT_CN.md`（先读这个）
- 合约：`solana-deposit/`（Anchor 0.31.1，8 个 localnet 测试全过）
- 监听器：`solana-watcher/`（Rust，10 个单测全过，clippy 零警告）
- Go 接口：`GET /api/v1/user/deposit-config`（`backend/internal/handler/deposit_handler.go`，含单测）
- 前端：`frontend/src/features/solana-wallet/`（CryptoDepositCard + useSolanaWallet，1237 测试全过）

## 2. 当前进度（接手从这里继续）

**devnet 部署和初始化已完成**：

- Program ID：`ErRaZ4rnCLC3nZdwwHuUTtgqnDD3UFGCsBDvsxii1X3i`
- ProgramData：`8RNuGABeow3wFT4em5xTBaDQ9HmVQTT7CmMz5Fu2k75Y`
- Upgrade authority / admin：`CGPod6DS9QUYD5p7UAtZWNGVD5ZXDFCgeqMKzH3NqX1S`
- 部署交易：`3TZZe58bf8qqdq5MDzwTewA6JcVkzirkw7K2KiC2qAGXekUzrqWsLnZ55R5eiRVVZCfbYv3JzLNfoEPymx9hzm8a`
- Config PDA：`FXp6JVXe2DxQX9JjyoYRACSDoB7D46sPhnpehtdj7Ar5`
- Vault PDA：`FaXEundahs9KTxwQfd856emUKa3w74C2T4w7gtwpLy9Y`
- 初始化交易：`3RSY8D2pTRhJy5v1ANaVM73CjNR1W6ZPf73D7NUxizZd9oCNKockd1PjN57R9aWdnpkcoTgqDFouDsau1u27bJBM`（finalized）
- 初始化参数：最低 `0.001 SOL` / `0.5 USDC`，devnet USDC mint
- 部署 buffer 已被消费并关闭；钱包剩余约 `3.02 SOL`

watcher 的 Dockerfile 已实际构建成功（本机 arm64 镜像约 35 MB），并通过 8 秒容器烟测：能启动并读取该 Program 的 devnet 签名。Rust 10 个单测和 clippy 也已重新通过。

Netcup（SSH host `summi-netcup`）上线已完成：

- 部署目录：`/opt/eyami/ops/eyami`
- sub2api 镜像：`sub2api:solana-devnet-20260722`，容器健康、0 次重启
- watcher 镜像：`sub2api-solana-watcher:solana-devnet-20260722`，持续运行、0 次重启
- watcher RPC：`https://solana-devnet.api.onfinality.io/public`
- watcher 状态库：Docker volume `eyami_eyami_solana_watcher_data`，已成功创建 SQLite 文件并读取到 2 条合约签名
- Admin API Key：从服务器 `sub2api` 数据库直接注入 `/opt/eyami/ops/eyami/env/solana-watcher.env`，文件权限 `600/root:root`；密钥未下载或打印
- 数据库备份：`/opt/eyami/backups/sub2api-before-solana-20260721T175641Z.dump`
- Compose 备份：`/opt/eyami/ops/eyami/docker-compose.yml.bak-20260721T175648Z-solana`
- sub2api 环境备份：`/opt/eyami/ops/eyami/env/sub2api.env.bak-20260721T175648Z-solana`

服务器内 `/health` 返回 200。新接口未登录返回 401（此前为 404），确认路由已经上线；公网域名先经过 Eyami 登录网关，因此未登录返回 302。

初始化/提款脚本原先漏声明直接依赖 `@solana/web3.js`，在 pnpm 严格模式下会报 `Cannot find module`；现已补入 `solana-deposit/package.json` 与锁文件，并通过 frozen install、Prettier 和 Node 语法检查。

**当前唯一未验证项**：尚未代表真实用户发起 devnet 充值，因此充值事件、自动兑换入账和提款的完整端到端链路仍需人工钱包操作确认。

## 3. 初始化合约

合约已经初始化。下列命令现在用于幂等复查：

```bash
cd solana-deposit
node scripts/initialize.js
# 默认参数：devnet USDC mint + 最低 0.001 SOL / 0.5 USDC，可用环境变量覆盖（见脚本头注释）
```

脚本幂等：已初始化会直接退出。成功输出 config/vault PDA 地址和 tx 签名。

## 4. 上线与验证

Netcup 已按等价配置上线，不使用仓库本地的 `deploy/.env`；Eyami 的实际配置位于 `/opt/eyami/ops/eyami/env/`。以下内容保留给新环境部署使用。

1. 管理后台 → 设置 → Admin API Key，生成 `admin-<64hex>`
2. `deploy/.env` 填入（`.env.example` 有完整注释）：

```bash
SOLANA_DEPOSIT_ENABLED=true
SOLANA_CLUSTER=devnet
SOLANA_PROGRAM_ID=ErRaZ4rnCLC3nZdwwHuUTtgqnDD3UFGCsBDvsxii1X3i
SOLANA_USDC_MINT=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
SOLANA_MIN_DEPOSIT_USD=1.0
SOLANA_RPC_HTTP_URL=https://solana-devnet.api.onfinality.io/public
SOLANA_WATCHER_ADMIN_API_KEY=admin-<64hex>
```

3. 启动：`cd deploy && docker compose --profile solana up -d`，看 `docker compose logs -f solana-watcher`
4. 验证：前端充值页出现"链上充值"卡片 → 连 devnet 钱包充 0.01 SOL → 1 分钟内余额到账
   （watcher 日志可见事件与入账记录；重复提交同 tx 不会重复入账）
5. 提款测试：`cd solana-deposit && TOKEN=sol AMOUNT=<lamports> RECIPIENT=<地址> node scripts/withdraw.js`

## 5. 环境关键信息（踩过的坑，别再踩）

- **网络**：本机直连 GitHub 超时，必须走代理 `http://127.0.0.1:7897`（Clash）
- **devnet 官方 RPC 限流**：本次部署时 `https://api.devnet.solana.com` 持续 429；读写改用 `https://solana-devnet.api.onfinality.io/public` 后成功
- **TPU 被墙**：Solana 交易默认走 TPU/QUIC（UDP），本网络不通 → 所有发交易的操作加 `--use-rpc`
- **Colima / Docker Hub**：容器内宿主代理地址为 `http://192.168.5.2:7897`；已在 Colima VM 的 `/etc/docker/daemon.json` 配置 daemon proxy。BuildKit 的远程 metadata resolver 仍可能绕过代理，必要时先 `docker pull` 两个基础镜像再 build
- **anchor test 要关代理**：本地 validator 在 127.0.0.1:8899，带着 http_proxy 会 502 → 跑 test 前 `unset http_proxy https_proxy`
- **CLI 空投限流**：`solana airdrop` 一直 429，用网页水龙头 https://faucet.solana.com
- **Rust 依赖降级**：`solana-deposit/Cargo.lock` 固化了 hybrid-array/indexmap/borsh/proc-macro-crate/zeroize*/blake3/unicode-segmentation 的旧版本（SBF 工具链 rustc 1.79 不认 edition2024 和新 MSRV）。**不要动这个 lock**，升级 anchor 时才重新评估
- **兑换码 code 上限 32 字符**：watcher 用 `sol_` + sha256(txsig) 前 28 hex 凑满 32，这是幂等键，格式不能改

## 6. 密钥位置与权限

| 密钥 | 位置 | 权限 |
|---|---|---|
| 部署者/admin 钱包 | `~/.config/solana/id.json`（`CGPod6DS9QUYD5p7UAtZWNGVD5ZXDFCgeqMKzH3NqX1S`） | 合约 upgrade authority + admin（提款权），devnet 余额约 3.02 SOL |
| 合约 keypair | `solana-deposit/target/deploy/solana_deposit-keypair.json` | Program ID `ErRaZ4rnCLC3nZdwwHuUTtgqnDD3UFGCsBDvsxii1X3i` |

watcher / sub2api 服务器不接触任何私钥。mainnet 部署前建议新建独立 admin 密钥并备份助记词。
本次部署使用过的 `/tmp/buffer-signer.json` 与 `/tmp/buffer2.json` 均已在确认链上账户关闭后删除。

## 7. 未做 / 后续迭代

- mainnet 部署（需真 SOL ~2-4 个付租金；devnet 验证通过后再做，命令相同换 cluster 和 USDC mint `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`）
- devnet 端到端充值/自动入账/提款验证（服务端已就绪，需要已登录测试用户和 devnet 钱包实际发起交易）
- user_id 链上公开可见；如需隐私改 ref_id 映射方案（见 docs 说明）
- 链上充值不生成 payment_orders，后台订单列表看不到；要可见需实现 `payment.Provider` 接口
