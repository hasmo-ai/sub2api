# Solana 链上充值接入指南

Sub2API 支持 Solana 链上充值：用户在前端连接 Solana 钱包（Phantom / Solflare 等），
直接向链上充值合约转入 SOL / USDC，链上确认后余额自动到账。

## 架构

```
前端充值页（连接钱包）→ 构造 deposit 交易 → 钱包签名上链
Anchor 合约 solana-deposit：资金进 Vault PDA，发出 DepositEvent{user_id, token, amount}
solana-watcher（Rust）：监听事件（finalized）→ 折算 USD → 调 Admin API 入账
管理员：anchor/脚本调 withdraw 指令把 Vault 资金提走
```

- 合约源码：`solana-deposit/`（Anchor 0.31.1，构建 `anchor build`，测试 `anchor test`）
- 监听服务：`solana-watcher/`（Rust，独立部署）
- 入账幂等：watcher 本地 SQLite 记录 + Admin API 兑换码 `code` 去重（双保险）

## 部署步骤

### 1. 部署合约

```bash
cd solana-deposit
pnpm install
anchor build

# devnet 示例
solana config set --url https://api.devnet.solana.com
solana-keygen new -o ~/solana-deposit-admin.json   # 运营密钥，妥善保管
solana airdrop 2 <ADMIN_PUBKEY>
anchor deploy --provider.cluster devnet --provider.wallet ~/solana-deposit-admin.json
```

记下 Program Id（如与 `declare_id!` 不一致，执行 `anchor keys sync` 后重新 build + deploy）。

初始化（设置最低充值额，示例：0.001 SOL / 0.5 USDC）：

```bash
# 用 anchor ts-node 或任意脚本调用：
# initialize(min_deposit_lamports=1_000_000, min_deposit_usdc=500_000)
# accounts: admin(签名), usdc_mint(devnet: 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU)
```

### 2. 配置 Sub2API

在管理后台 **设置 → Admin API Key** 生成密钥（格式 `admin-<64hex>`）。

环境变量（docker 部署写入 `deploy/.env`）：

```bash
SOLANA_DEPOSIT_ENABLED=true
SOLANA_CLUSTER=devnet                 # 或 mainnet-beta
SOLANA_PROGRAM_ID=<上一步的 Program Id>
SOLANA_USDC_MINT=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU   # mainnet: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
SOLANA_MIN_DEPOSIT_USD=1.0

SOLANA_RPC_HTTP_URL=https://api.devnet.solana.com   # 生产建议用付费 RPC
SOLANA_WATCHER_ADMIN_API_KEY=admin-<64hex>
```

### 3. 启动

```bash
docker compose --profile solana up -d
docker compose logs -f solana-watcher
```

前端充值页会自动出现"链上充值"卡片。

## 验证流程（devnet）

1. 前端连接 devnet 钱包（钱包需切到 devnet 并有空投 SOL）
2. 充值页选择 SOL/USDC，输入金额，确认交易
3. 观察 `solana-watcher` 日志：约 1 分钟内看到 deposit 事件并入账
4. 用户余额增加；重复提交同一笔交易不会重复入账（幂等）
5. 提款：`withdraw_sol` / `withdraw_usdc`（仅 admin 密钥可调）

## 注意

- 只认 `finalized` 交易；失败交易不入账
- SOL 价格取 CoinGecko，价格源不可用时暂停 SOL 入账（USDC 1:1 不受影响）
- 低于 `SOLANA_MIN_DEPOSIT_USD` 的充值只记录不入账
- user_id 在链上事件中公开可见
- 只有 admin 密钥能从 Vault 提款；服务器（watcher / sub2api）不接触任何私钥
