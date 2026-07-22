# solana-deposit

sub2api 的 Solana 链上充值合约（Anchor 程序）。用户调用 `deposit_sol` / `deposit_usdc`
将 SOL / USDC 存入程序 Vault PDA，程序发出 `DepositEvent{user_id, token, amount}`；
链下 `solana-watcher` 服务监听事件并调用 sub2api Admin API 为用户加余额。
管理员（部署者密钥）可通过 `withdraw_sol` / `withdraw_usdc` 将 Vault 资金提走。

## 指令

| 指令 | 说明 | 权限 |
|------|------|------|
| `initialize(min_lamports, min_usdc)` | 初始化 Config/Vault PDA 与 Vault USDC ATA | 首次部署，签名者成为 admin |
| `deposit_sol(amount, user_id)` | 存入 SOL（lamports），发出 DepositEvent | 任意用户 |
| `deposit_usdc(amount, user_id)` | 存入 USDC（6 位小数），发出 DepositEvent | 任意用户 |
| `withdraw_sol(amount)` | Vault → 指定地址 | 仅 admin |
| `withdraw_usdc(amount)` | Vault ATA → 指定 ATA | 仅 admin |
| `update_config(min_lamports, min_usdc)` | 调整最低充值金额 | 仅 admin |

## 构建与测试

```bash
pnpm install        # 安装 TS 测试依赖
anchor build
anchor test         # 本地 validator 下跑完整测试
```

## 部署（devnet）

```bash
# 1. 准备运营密钥（即 admin，妥善保管；建议使用独立密钥而非默认 id.json）
solana-keygen new -o ~/solana-deposit-admin.json

# 2. 切到 devnet 并领取测试 SOL
solana config set --url https://api.devnet.solana.com
solana airdrop 2 <ADMIN_PUBKEY>

# 3. 部署
anchor deploy --provider.cluster devnet --provider.wallet ~/solana-deposit-admin.json
# 记下输出的 Program Id，执行 anchor keys sync 保持 declare_id! 一致后重新 build/deploy

# 4. 初始化（devnet USDC mint: 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU）
#    最低充值示例：0.001 SOL / 0.5 USDC
anchor run --help  # 或使用 scripts/ 下的初始化脚本 / anchor ts-node 调用 initialize
```

## 与 sub2api 的对接

1. 把 Program Id 填入 sub2api 配置：`SOLANA_DEPOSIT_ENABLED=true`、`SOLANA_CLUSTER`、
   `SOLANA_PROGRAM_ID`、`SOLANA_USDC_MINT`
2. 启动 `solana-watcher`（`docker compose --profile solana up -d`），它会监听
   本程序的事件并自动入账
3. 前端充值页自动出现"链上充值"入口

## 注意

- user_id 在链上事件中公开可见，请勿把链上数据当作隐私数据处理
- 只有 admin 密钥能从 Vault 提款；watcher 与 sub2api 服务器均不接触私钥
- mainnet 部署前请将 `Anchor.toml` 的 cluster 与钱包路径切换为正式配置，
  并考虑把 admin 迁移到硬件钱包或多签
