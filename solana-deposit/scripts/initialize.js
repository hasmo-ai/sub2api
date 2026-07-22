// Initialize the solana-deposit program on the cluster specified by env.
// Usage:
//   CLUSTER_RPC=https://api.devnet.solana.com WALLET=~/.config/solana/id.json \
//   MIN_SOL_LAMPORTS=1000000 MIN_USDC_UNITS=500000 \
//   USDC_MINT=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU \
//   node scripts/initialize.js
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
const MIN_SOL_LAMPORTS = new anchor.BN(
  process.env.MIN_SOL_LAMPORTS || "1000000"
); // 0.001 SOL
const MIN_USDC_UNITS = new anchor.BN(process.env.MIN_USDC_UNITS || "500000"); // 0.5 USDC
const USDC_MINT = new PublicKey(
  process.env.USDC_MINT || "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU" // devnet USDC
);

async function main() {
  const keypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(WALLET, "utf8")))
  );
  const connection = new Connection(CLUSTER_RPC, "confirmed");
  const wallet = new anchor.Wallet(keypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  const program = new anchor.Program(idl, provider);

  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault")],
    program.programId
  );

  const existing = await connection.getAccountInfo(configPda);
  if (existing) {
    console.log("config already initialized:", configPda.toBase58());
    return;
  }

  const sig = await program.methods
    .initialize(MIN_SOL_LAMPORTS, MIN_USDC_UNITS)
    .accounts({ admin: wallet.publicKey, usdcMint: USDC_MINT })
    .rpc();

  console.log("initialized!");
  console.log("  program :", program.programId.toBase58());
  console.log("  config  :", configPda.toBase58());
  console.log("  vault   :", vaultPda.toBase58());
  console.log("  admin   :", wallet.publicKey.toBase58());
  console.log("  usdcMint:", USDC_MINT.toBase58());
  console.log("  tx      :", sig);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
