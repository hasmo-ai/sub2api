import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { assert, expect } from "chai";
import { SolanaDeposit } from "../target/types/solana_deposit";

describe("solana-deposit", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaDeposit as Program<SolanaDeposit>;
  const admin = provider.wallet as anchor.Wallet;
  const connection = provider.connection;

  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault")],
    program.programId
  );

  const MIN_SOL = new BN(LAMPORTS_PER_SOL / 100); // 0.01 SOL
  const MIN_USDC = new BN(1_000_000); // 1 USDC (6 decimals)

  let usdcMint: PublicKey;
  const user = Keypair.generate();
  const USER_ID = new BN(42);

  const confirm = async (sig: string) => {
    const latest = await connection.getLatestBlockhash();
    await connection.confirmTransaction({ signature: sig, ...latest });
  };

  const airdrop = async (to: PublicKey, sol: number) => {
    await confirm(await connection.requestAirdrop(to, sol * LAMPORTS_PER_SOL));
  };

  const nextDepositEvent = (): Promise<{
    userId: BN;
    token: object;
    amount: BN;
  }> =>
    new Promise((resolve) => {
      const id = program.addEventListener("depositEvent", (event) => {
        program.removeEventListener(id);
        resolve(event);
      });
    });

  before(async () => {
    await airdrop(user.publicKey, 2);
    usdcMint = await createMint(
      connection,
      admin.payer,
      admin.publicKey,
      null,
      6
    );
  });

  it("initializes config, vault and vault USDC ATA", async () => {
    await program.methods
      .initialize(MIN_SOL, MIN_USDC)
      .accounts({ admin: admin.publicKey, usdcMint })
      .rpc();

    const config = await program.account.config.fetch(configPda);
    assert.ok(config.admin.equals(admin.publicKey));
    assert.ok(config.usdcMint.equals(usdcMint));
    assert.ok(config.minDepositLamports.eq(MIN_SOL));
    assert.ok(config.minDepositUsdc.eq(MIN_USDC));

    const vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultPda, true);
    const ataInfo = await getAccount(connection, vaultAta);
    assert.equal(Number(ataInfo.amount), 0);
  });

  it("accepts a SOL deposit and emits DepositEvent", async () => {
    const amount = new BN(LAMPORTS_PER_SOL / 10); // 0.1 SOL
    const before = await connection.getBalance(vaultPda);
    const eventPromise = nextDepositEvent();

    await program.methods
      .depositSol(amount, USER_ID)
      .accounts({ payer: user.publicKey })
      .signers([user])
      .rpc();

    const after = await connection.getBalance(vaultPda);
    assert.equal(after - before, amount.toNumber());

    const event = await eventPromise;
    assert.ok(event.userId.eq(USER_ID));
    assert.ok(event.amount.eq(amount));
    assert.deepEqual(event.token, { sol: {} });
  });

  it("rejects a SOL deposit below the minimum", async () => {
    try {
      await program.methods
        .depositSol(new BN(1), USER_ID)
        .accounts({ payer: user.publicKey })
        .signers([user])
        .rpc();
      assert.fail("should have thrown");
    } catch (err) {
      expect(err.toString()).to.include("BelowMinimum");
    }
  });

  it("accepts a USDC deposit and emits DepositEvent", async () => {
    const userAta = await getOrCreateAssociatedTokenAccount(
      connection,
      admin.payer,
      usdcMint,
      user.publicKey
    );
    await mintTo(
      connection,
      admin.payer,
      usdcMint,
      userAta.address,
      admin.payer,
      10_000_000
    );

    const amount = new BN(5_000_000); // 5 USDC
    const eventPromise = nextDepositEvent();

    await program.methods
      .depositUsdc(amount, USER_ID)
      .accounts({ payer: user.publicKey, usdcMint })
      .signers([user])
      .rpc();

    const vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultPda, true);
    const vaultAccount = await getAccount(connection, vaultAta);
    assert.equal(Number(vaultAccount.amount), 5_000_000);

    const event = await eventPromise;
    assert.ok(event.userId.eq(USER_ID));
    assert.ok(event.amount.eq(amount));
    assert.deepEqual(event.token, { usdc: {} });
  });

  it("rejects SOL withdrawal from a non-admin", async () => {
    const stranger = Keypair.generate();
    await airdrop(stranger.publicKey, 1);
    try {
      await program.methods
        .withdrawSol(new BN(1_000))
        .accounts({
          admin: stranger.publicKey,
          recipient: stranger.publicKey,
        })
        .signers([stranger])
        .rpc();
      assert.fail("should have thrown");
    } catch (err) {
      expect(err.toString()).to.include("Unauthorized");
    }
  });

  it("allows the admin to withdraw SOL", async () => {
    const recipient = Keypair.generate().publicKey;
    const amount = new BN(LAMPORTS_PER_SOL / 20);
    await program.methods
      .withdrawSol(amount)
      .accounts({ admin: admin.publicKey, recipient })
      .rpc();

    const balance = await connection.getBalance(recipient);
    assert.equal(balance, amount.toNumber());
  });

  it("allows the admin to withdraw USDC", async () => {
    const recipientAta = await getOrCreateAssociatedTokenAccount(
      connection,
      admin.payer,
      usdcMint,
      admin.publicKey
    );
    const amount = new BN(2_000_000);
    await program.methods
      .withdrawUsdc(amount)
      .accounts({
        admin: admin.publicKey,
        usdcMint,
        recipientUsdcAta: recipientAta.address,
      })
      .rpc();

    const account = await getAccount(connection, recipientAta.address);
    assert.equal(Number(account.amount), 2_000_000);
  });

  it("lets the admin update min deposit thresholds", async () => {
    await program.methods
      .updateConfig(new BN(2_000_000), new BN(2_000_000))
      .accounts({ admin: admin.publicKey })
      .rpc();
    const config = await program.account.config.fetch(configPda);
    assert.ok(config.minDepositLamports.eq(new BN(2_000_000)));
    assert.ok(config.minDepositUsdc.eq(new BN(2_000_000)));
  });
});
