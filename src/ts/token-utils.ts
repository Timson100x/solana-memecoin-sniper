import {
  PublicKey, Connection,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";

export const TOKEN_2022_PK = new PublicKey(
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
);

// Erkennt ob Mint Token-2022 oder Legacy ist
export async function detectTokenProgram(
  connection: Connection,
  mint: PublicKey
): Promise<PublicKey> {
  const info = await connection.getAccountInfo(mint, "processed");
  if (!info) throw new Error(`Mint ${mint.toBase58()} nicht gefunden`);
  // Token-2022 Programm ist Owner des Mint-Accounts
  return info.owner.equals(TOKEN_2022_PK)
    ? TOKEN_2022_PK
    : TOKEN_PROGRAM_ID;
}

// Gibt ATA-Adresse + TokenProgram zurück
export function getATAForProgram(
  mint: PublicKey,
  owner: PublicKey,
  tokenProgram: PublicKey
): PublicKey {
  return getAssociatedTokenAddressSync(mint, owner, false, tokenProgram);
}

// Gibt ATA für BondingCurve (allowOwnerOffCurve=true) zurück
export function getCurveATA(
  mint: PublicKey,
  curve: PublicKey,
  tokenProgram: PublicKey
): PublicKey {
  return getAssociatedTokenAddressSync(mint, curve, true, tokenProgram);
}

// Prüft ob ATA existiert, erstellt Instruction falls nicht
export async function ensureATAInstruction(
  connection:   Connection,
  payer:        PublicKey,
  mint:         PublicKey,
  owner:        PublicKey,
  tokenProgram: PublicKey
) {
  const ata     = getATAForProgram(mint, owner, tokenProgram);
  const ataInfo = await connection.getAccountInfo(ata, "processed");
  return {
    ata,
    instruction: ataInfo ? null : createAssociatedTokenAccountInstruction(
      payer, ata, owner, mint, tokenProgram
    ),
  };
}
