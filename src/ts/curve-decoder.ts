import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

export interface BondingCurveState {
  virtualTokenReserves: BN;
  virtualSolReserves:   BN;
  realTokenReserves:    BN;
  realSolReserves:      BN;
  tokenTotalSupply:     BN;
  complete:             boolean;
  creator:              PublicKey | null;
  isMayhemMode:         boolean;
  // cashback_enabled kommt nach mayhem — für spätere Erweiterung
}

// Layout (Pump.fun Mainnet, April 2026):
// Offset | Size | Field
// 0      | 8    | discriminator (Anchor)
// 8      | 8    | virtual_token_reserves  u64 LE
// 16     | 8    | virtual_sol_reserves    u64 LE
// 24     | 8    | real_token_reserves     u64 LE
// 32     | 8    | real_sol_reserves       u64 LE
// 40     | 8    | token_total_supply      u64 LE
// 48     | 1    | complete                bool
// 49     | 32   | creator                 Pubkey
// 81     | 1    | is_mayhem_mode          bool
// 82+    | ?    | reserved / cashback_enabled etc.

export function decodeCurveState(
  raw: Buffer | Uint8Array,
  hasDiscriminator: boolean = true
): BondingCurveState | null {
  try {
    const buf = Buffer.from(raw);
    let off   = hasDiscriminator ? 8 : 0;

    if (off + 41 > buf.length) return null;

    const r64 = (): BN => {
      const v = new BN(buf.slice(off, off + 8), "le");
      off += 8;
      return v;
    };

    const virtualTokenReserves = r64();
    const virtualSolReserves   = r64();
    const realTokenReserves    = r64();
    const realSolReserves      = r64();
    const tokenTotalSupply     = r64();

    // Plausibilitäts-Guard
    if (virtualSolReserves.isZero() || virtualTokenReserves.isZero()) return null;

    const complete = buf[off++] === 1;

    // Creator (32 Bytes) — optional, nur wenn Daten vorhanden
    let creator: PublicKey | null = null;
    if (off + 32 <= buf.length) {
      creator = new PublicKey(buf.slice(off, off + 32));
      off += 32;
    }

    // is_mayhem_mode (Byte 81 im vollen Buffer, Byte 73 im Slice ab offset=8)
    let isMayhemMode = false;
    if (off < buf.length) {
      isMayhemMode = buf[off] === 1;
      off++;
    }

    return {
      virtualTokenReserves,
      virtualSolReserves,
      realTokenReserves,
      realSolReserves,
      tokenTotalSupply,
      complete,
      creator,
      isMayhemMode,
    };
  } catch { return null; }
}

// dataSlice-Version: offset=8 übersprungen → hasDiscriminator=false
export function decodeCurveSlice(raw: Buffer | Uint8Array): BondingCurveState | null {
  return decodeCurveState(raw, false);
}
