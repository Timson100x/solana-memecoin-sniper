// presale-validator.ts
// Discriminator-Berechnung für m8s-lab Fork
// anchor idl fetch Cu3ZCXsVh7xC64gWH23vjDeytWC6ZGcMRVYZAka92QTq

import { createHash } from "crypto";

// Anchor Discriminator = sha256("global:<instruction_name>")[0:8]
function anchorDisc(instructionName: string): Buffer {
  const hash = createHash("sha256")
    .update(`global:${instructionName}`)
    .digest();
  return hash.slice(0, 8);
}

// Berechne alle relevanten Discriminatoren für m8s-lab
export const M8S_DISC = {
  CREATE:     anchorDisc("create"),
  BUY:        anchorDisc("buy"),
  SELL:       anchorDisc("sell"),
  SET_PARAMS: anchorDisc("set_params"),   // Presale-Config
  INITIALIZE: anchorDisc("initialize"),
};

// Verify gegen bekannte Werte aus IDL
function verify() {
  const expected = {
    CREATE:  [24,  30, 200,  40,   5,  28,   7, 119],
    BUY:     [102,  6,  61,  18,   1, 218, 235, 234],
    SELL:    [51,  230, 133, 164,   1, 127, 131, 173],
  };

  for (const [name, bytes] of Object.entries(expected)) {
    const computed = M8S_DISC[name as keyof typeof M8S_DISC];
    const match    = bytes.every((b, i) => computed[i] === b);
    console.log(`Discriminator ${name}: ${match ? "✓" : "✗ MISMATCH"}`);
    if (!match) {
      console.log(`  Expected: [${bytes}]`);
      console.log(`  Got:      [${[...computed]}]`);
    }
  }
  console.log(`SET_PARAMS: [${[...M8S_DISC.SET_PARAMS]}]`);
}

// Bei Start ausführen um Konsistenz zu prüfen
verify();

export function decodePresaleInstruction(data: Buffer): {
  presaleAmount: import("bn.js"); presaleTime: number
} | null {
  if (!data.slice(0, 8).equals(M8S_DISC.SET_PARAMS)) return null;
  try {
    if (data.length < 24) return null;
    const { BN } = require("@coral-xyz/anchor");
    const presaleAmount = new BN(data.slice(8,  16), "le");
    const presaleTime   = Number(data.readBigInt64LE(16));
    return { presaleAmount, presaleTime };
  } catch { return null; }
}
