// Admin withdrawal from the vault (admin keypair required).
// Usage:
//   # 提取 0.5 SOL 到指定地址
//   TOKEN=sol AMOUNT=500000000 RECIPIENT=<地址> node scripts/withdraw.js
//   # 提取 100 USDC 到指定 ATA
//   TOKEN=usdc AMOUNT=100000000 RECIPIENT=<ATA地址> node scripts/withdraw.js
// 可选环境变量：CLUSTER_RPC（默认 devnet）、WALLET（默认 ~/.config/solana/id.json）
const anchor = require("@coral-xyz/anchor");
const { Connection, Keypair, PublicKey } = require("@solana/web3.js");
const fs = require("fs");
const os = require("os");
const idl = require("../target/idl/solana_deposit.json");

const CLUSTER_RPC = process.env.CLUSTER_RPC || "https://api.devnet.solana.com";
const WALLET = (process.env.WALLET || "~/.config/solana/id.json").replace(
  /^~/,
  os.homedir()
);
const TOKEN = (process.env.TOKEN || "sol").toLowerCase();
const AMOUNT = process.env.AMOUNT;
const RECIPIENT = process.env.RECIPIENT;

async function main() {
  if (!AMOUNT || !RECIPIENT) {
    console.error("需要 AMOUNT（最小单位整数）和 RECIPIENT 环境变量");
    process.exit(1);
  }
  const keypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(WALLET, "utf8")))
  );
  const connection = new Connection(CLUSTER_RPC, "confirmed");
  const wallet = new anchor.Wallet(keypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  const program = new anchor.Program(idl, provider);
  const amount = new anchor.BN(AMOUNT);

  let sig;
  if (TOKEN === "sol") {
    sig = await program.methods
      .withdrawSol(amount)
      .accounts({
        admin: wallet.publicKey,
        recipient: new PublicKey(RECIPIENT),
      })
      .rpc();
  } else if (TOKEN === "usdc") {
    const config = await program.account.config.fetch(
      PublicKey.findProgramAddressSync(
        [Buffer.from("config")],
        program.programId
      )[0]
    );
    sig = await program.methods
      .withdrawUsdc(amount)
      .accounts({
        admin: wallet.publicKey,
        usdcMint: config.usdcMint,
        recipientUsdcAta: new PublicKey(RECIPIENT),
      })
      .rpc();
  } else {
    console.error("TOKEN 只能是 sol 或 usdc");
    process.exit(1);
  }
  console.log(`withdraw ${TOKEN} ok, tx:`, sig);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
