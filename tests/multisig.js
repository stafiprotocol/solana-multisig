const anchor = require("@project-serum/anchor");
const assert = require("assert");

describe("multisig", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.Multisig;

  it("Tests the multisig program", async () => {
    const localAccount = anchor.web3.Keypair.fromSecretKey(
      new Uint8Array([179, 95, 213, 234, 125, 167, 246, 188, 230, 134, 181, 219, 31, 146, 239, 75, 190, 124, 112, 93, 187, 140, 178, 119, 90, 153, 207, 178, 137, 5, 53, 71, 116, 28, 190, 12, 249, 238, 110, 135, 109, 21, 196, 36, 191, 19, 236, 175, 229, 204, 68, 180, 130, 102, 71, 239, 41, 53, 152, 159, 175, 124, 180, 6])
    )
    console.log("myLocal account:", localAccount.publicKey.toString());

    const multisig = anchor.web3.Keypair.generate();
    console.log("multisig account:", multisig.publicKey.toString());

    const [
      multisigSigner,
      nonce,
    ] = await anchor.web3.PublicKey.findProgramAddress(
      [multisig.publicKey.toBuffer()],
      program.programId
    );
    const multisigSize = 200; // Big enough.

    const ownerA = anchor.web3.Keypair.generate();
    const ownerB = anchor.web3.Keypair.generate();
    const ownerC = anchor.web3.Keypair.generate();
    const ownerD = anchor.web3.Keypair.generate();
    const owners = [ownerA.publicKey, ownerB.publicKey, ownerC.publicKey];
    console.log("ownerA:", ownerA.publicKey.toString());
    console.log("ownerB:", ownerB.publicKey.toString());
    console.log("ownerC:", ownerC.publicKey.toString());
    console.log("multiSigner:", multisigSigner.toString());


    //creste multisig
    const threshold = new anchor.BN(3);
    await program.rpc.createMultisig(owners, threshold, nonce, {
      accounts: {
        multisig: multisig.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [
        await program.account.multisig.createInstruction(
          multisig,
          multisigSize
        ),
      ],
      signers: [multisig],
    });


    let multisigAccount = await program.account.multisig(multisig.publicKey);
    assert.strictEqual(multisigAccount.nonce, nonce);
    assert.ok(multisigAccount.threshold.eq(new anchor.BN(3)));
    assert.deepStrictEqual(multisigAccount.owners, owners);

    const accounts = [
      {
        pubkey: multisigSigner,
        isWritable: true,
        isSigner: true,
      },
      {
        pubkey: ownerA.publicKey,
        isWritable: true,
        isSigner: false,
      },
    ];
    const pid = anchor.web3.SystemProgram.programId

    const data = anchor.web3.SystemProgram.transfer({
      fromPubkey: multisigSigner,
      toPubkey: ownerA.publicKey,
      lamports: new anchor.BN(1000000000),
    }).data
    const data2 = anchor.web3.SystemProgram.transfer({
      fromPubkey: multisigSigner,
      toPubkey: ownerA.publicKey,
      lamports: new anchor.BN(100000000),
    }).data

    await transfer(provider, localAccount.publicKey, multisigSigner, localAccount);

    //create transaction account 
    const transaction = anchor.web3.Keypair.generate();
    const txSize = 1000; // Big enough, cuz I'm lazy.
    const createAccountInstruction = anchor.web3.SystemProgram.createAccount({
      fromPubkey: localAccount.publicKey,
      newAccountPubkey: transaction.publicKey,
      space: 1000, // Add 8 for the account discriminator.
      lamports: 1000000000,
      programId: program.programId,
    });
    const tx = new anchor.web3.Transaction();
    tx.add(...[createAccountInstruction]);
    await provider.send(tx, [localAccount, transaction]);
    console.log("transaction account:", transaction.publicKey.toString());


    //create transaction 
    await program.rpc.createTransaction([pid,pid], [accounts,accounts], [data,data2], {
      accounts: {
        multisig: multisig.publicKey,
        transaction: transaction.publicKey,
        proposer: ownerA.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [ownerA],
    });

    const txAccount = await program.account.transaction(transaction.publicKey);

    assert.ok(txAccount.programId[0].equals(pid));
    assert.deepStrictEqual(txAccount.accounts[0], accounts);
    assert.deepStrictEqual(txAccount.data[0], data);
    assert.ok(txAccount.multisig.equals(multisig.publicKey));
    assert.deepStrictEqual(txAccount.didExecute, false);

    // Now that we've reached the threshold, send the transactoin.
    await program.rpc.approve({
      accounts: {
        multisig: multisig.publicKey,
        multisigSigner,
        transaction: transaction.publicKey,
        owner: ownerB.publicKey,
      },
      remainingAccounts: [
        {
          pubkey: multisigSigner,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: ownerA.publicKey,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: anchor.web3.SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },

        {
          pubkey: program.programId,
          isWritable: false,
          isSigner: false,
        }],
      signers: [ownerB]
    });

    await program.rpc.approve({
      accounts: {
        multisig: multisig.publicKey,
        multisigSigner,
        transaction: transaction.publicKey,
        owner: ownerC.publicKey,
      },
      remainingAccounts: [
        {
          pubkey: multisigSigner,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: ownerA.publicKey,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: anchor.web3.SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },

        {
          pubkey: program.programId,
          isWritable: false,
          isSigner: false,
        }],
      signers: [ownerC]
    });

  });
});



async function transfer(provider, from, to, authority) {

  const instructions = [anchor.web3.SystemProgram.transfer({
    fromPubkey: from,
    toPubkey: to,
    lamports: new anchor.BN(3000000000),
  }),
  ]

  const tx = new anchor.web3.Transaction();
  tx.add(...instructions);

  await provider.send(tx, [authority]);
  return;
}


