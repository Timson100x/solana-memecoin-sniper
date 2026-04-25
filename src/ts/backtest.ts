// backtest.ts
// Replayt historische Bonding-Curve-Snapshots und simuliert die v3.1-Logik
// Input: JSONL-Datei mit CurveSnapshot-Events (aus gRPC-Recording oder RPC-Dumps)

import * as fs from "fs";
import * as readline from "readline";
import { BN } from "@coral-xyz/anchor";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { decodeCurveState, BondingCurveState } from "./curve-decoder";
import { EventEmitter } from "events";

// ─── Config ───────────────────────────────────────────────────────────────────

interface BacktestConfig {
  BUY_AMOUNT_SOL:    number;
  MAX_SLIPPAGE_BPS:  number;
  TP_MULTIPLE:       number;
  SL_MULTIPLE:       number;
  TRAILING_STOP_BPS: number;
  MAX_MCAP_USD:      number;
  MIN_MCAP_USD:      number;
  MAX_HOLD_MS:       number;
  SOL_PRICE_USD:     number;
  FEE_BPS:           number;
}

const DEFAULT_CONFIG: BacktestConfig = {
  BUY_AMOUNT_SOL:    0.05,
  MAX_SLIPPAGE_BPS:  1500,
  TP_MULTIPLE:       2.5,
  SL_MULTIPLE:       0.5,
  TRAILING_STOP_BPS: 3000,
  MAX_MCAP_USD:      50_000,
  MIN_MCAP_USD:      5_000,
  MAX_HOLD_MS:       600_000,
  SOL_PRICE_USD:     150,
  FEE_BPS:           100,
};

// ─── Types ────────────────────────────────────────────────────────────────────

interface CurveSnapshot {
  ts:                    number;
  mint:                  string;
  virtualTokenReserves:  string;
  virtualSolReserves:    string;
  realTokenReserves:     string;
  realSolReserves:       string;
  tokenTotalSupply:      string;
  complete:              boolean;
  isLaunch:              boolean;
}

interface BacktestPosition {
  mint:          string;
  buyPrice:      number;
  tokenAmount:   BN;
  investedSOL:   number;
  openedAt:      number;
  highWaterMark: number;
  buyMcapUSD:    number;
}

interface TradeResult {
  mint:        string;
  buyMcapUSD:  number;
  sellMcapUSD: number;
  pnlSOL:      number;
  pnlX:        number;
  holdMs:      number;
  reason:      string;
  isPresale:   boolean;
}

// ─── BondingCurve Math (identisch zu live) ───────────────────────────────────

const BC = {
  priceSOL(s: BondingCurveState): number {
    return s.virtualSolReserves.toNumber() / s.virtualTokenReserves.toNumber();
  },

  priceUSD(s: BondingCurveState, solUSD: number): number {
    return (s.virtualSolReserves.toNumber() / LAMPORTS_PER_SOL)
         / s.virtualTokenReserves.toNumber()
         * solUSD;
  },

  mcapUSD(s: BondingCurveState, solUSD: number): number {
    return BC.priceUSD(s, solUSD) * s.tokenTotalSupply.toNumber();
  },

  tokensOut(s: BondingCurveState, solLamports: number): BN {
    const sol = new BN(solLamports);
    return s.virtualTokenReserves.mul(sol).div(s.virtualSolReserves.add(sol));
  },

  solOut(s: BondingCurveState, tokenIn: BN): number {
    return s.virtualSolReserves
      .mul(tokenIn)
      .div(s.virtualTokenReserves.add(tokenIn))
      .toNumber();
  },

  applyFee(lamports: number, feeBps: number): number {
    return Math.floor(lamports * (10_000 - feeBps) / 10_000);
  },
};

// ─── Backtester ───────────────────────────────────────────────────────────────

class Backtester extends EventEmitter {
  private config:    BacktestConfig;
  private positions  = new Map<string, BacktestPosition>();
  private results:   TradeResult[] = [];
  private skipped    = 0;
  private launched   = 0;

  constructor(config: BacktestConfig = DEFAULT_CONFIG) {
    super();
    this.config = config;
  }

  async run(inputFile: string): Promise<BacktestReport> {
    console.log(`[Backtest] Starte mit Config:`, this.config);
    console.log(`[Backtest] Input: ${inputFile}`);

    const rl = readline.createInterface({
      input: fs.createReadStream(inputFile),
      crlfDelay: Infinity,
    });

    let lineCount = 0;

    for await (const line of rl) {
      if (!line.trim()) continue;
      lineCount++;

      try {
        const snap: CurveSnapshot = JSON.parse(line);
        this.processSnapshot(snap);
      } catch (err: any) {
        console.warn(`[Backtest] Parse-Fehler Zeile ${lineCount}: ${err.message}`);
      }
    }

    for (const [mint, pos] of this.positions) {
      const state = this.mockStateFromPos(pos);
      this.closeTrade(mint, pos, state, "BACKTEST_END", pos.buyPrice);
    }

    return this.generateReport();
  }

  async sweep(
    inputFile: string,
    paramGrid: Partial<BacktestConfig>[]
  ): Promise<SweepResult[]> {
    const sweepResults: SweepResult[] = [];

    for (const params of paramGrid) {
      const cfg      = { ...DEFAULT_CONFIG, ...params };
      const tester   = new Backtester(cfg);
      const report   = await tester.run(inputFile);
      sweepResults.push({ config: cfg, report });
      console.log(
        `[Sweep] TP=${cfg.TP_MULTIPLE} SL=${cfg.SL_MULTIPLE} ` +
        `Trail=${cfg.TRAILING_STOP_BPS}bps → ` +
        `WinRate=${(report.winRate * 100).toFixed(1)}% ` +
        `PnL=${report.totalPnlSOL.toFixed(3)} SOL ` +
        `Sharpe=${report.sharpeRatio.toFixed(2)}`
      );
    }

    sweepResults.sort((a, b) => b.report.sharpeRatio - a.report.sharpeRatio);
    return sweepResults;
  }

  private processSnapshot(snap: CurveSnapshot) {
    const state = this.snapshotToState(snap);
    if (!state) return;

    if (snap.isLaunch) {
      this.launched++;
      this.trySnipe(snap.mint, snap.ts, state, false);
      return;
    }

    const pos = this.positions.get(snap.mint);
    if (!pos) return;

    this.onCurveUpdate(snap.mint, snap.ts, pos, state);
  }

  private trySnipe(
    mint:      string,
    ts:        number,
    state:     BondingCurveState,
    isPresale: boolean
  ) {
    if (this.positions.size >= 3) { this.skipped++; return; }
    if (state.complete) return;

    const mcapUSD = BC.mcapUSD(state, this.config.SOL_PRICE_USD);

    if (!isPresale) {
      if (mcapUSD < this.config.MIN_MCAP_USD || mcapUSD > this.config.MAX_MCAP_USD) {
        this.skipped++;
        return;
      }
    }

    const solIn    = Math.floor(this.config.BUY_AMOUNT_SOL * LAMPORTS_PER_SOL);
    const expected = BC.tokensOut(state, solIn);
    const netSolIn = BC.applyFee(solIn, this.config.FEE_BPS);

    this.positions.set(mint, {
      mint,
      buyPrice:      BC.priceSOL(state),
      tokenAmount:   expected,
      investedSOL:   this.config.BUY_AMOUNT_SOL,
      openedAt:      ts,
      highWaterMark: BC.priceSOL(state),
      buyMcapUSD:    mcapUSD,
    });
  }

  private onCurveUpdate(
    mint:  string,
    ts:    number,
    pos:   BacktestPosition,
    state: BondingCurveState
  ) {
    const currentPrice = BC.priceSOL(state);
    const pnlX         = currentPrice / pos.buyPrice;

    if (currentPrice > pos.highWaterMark) {
      pos.highWaterMark = currentPrice;
    }

    const trailingPrice = pos.highWaterMark * (1 - this.config.TRAILING_STOP_BPS / 10_000);
    const holdMs        = ts - pos.openedAt;
    const stale         = holdMs > this.config.MAX_HOLD_MS;

    const tp       = pnlX >= this.config.TP_MULTIPLE;
    const sl       = pnlX <= this.config.SL_MULTIPLE;
    const trailing = currentPrice <= trailingPrice && pnlX > 1.1;

    if (tp || sl || trailing || state.complete || stale) {
      const reason = tp        ? "TP"
                   : sl        ? "SL"
                   : trailing  ? "TRAILING"
                   : state.complete ? "CURVE_COMPLETE"
                   : "TIMEOUT";
      this.closeTrade(mint, pos, state, reason, pnlX);
    }
  }

  private closeTrade(
    mint:    string,
    pos:     BacktestPosition,
    state:   BondingCurveState,
    reason:  string,
    pnlX:    number
  ) {
    const rawSolOut = BC.solOut(state, pos.tokenAmount);
    const netSolOut = BC.applyFee(rawSolOut, this.config.FEE_BPS) / LAMPORTS_PER_SOL;
    const pnlSOL    = netSolOut - pos.investedSOL;
    const sellMcap  = BC.mcapUSD(state, this.config.SOL_PRICE_USD);

    this.results.push({
      mint,
      buyMcapUSD:  pos.buyMcapUSD,
      sellMcapUSD: sellMcap,
      pnlSOL,
      pnlX,
      holdMs:      Date.now() - pos.openedAt,
      reason,
      isPresale:   false,
    });

    this.positions.delete(mint);
  }

  private mockStateFromPos(pos: BacktestPosition): BondingCurveState {
    const priceRatio = pos.buyPrice;
    const vSol       = new BN(Math.floor(priceRatio * 1e9));
    const vToken     = new BN(1e9);
    return {
      virtualTokenReserves: vToken,
      virtualSolReserves:   vSol,
      realTokenReserves:    vToken,
      realSolReserves:      vSol,
      tokenTotalSupply:     new BN(1_000_000_000_000_000),
      complete:             false,
      creator:              null,
      isMayhemMode:         false,
    };
  }

  private snapshotToState(snap: CurveSnapshot): BondingCurveState | null {
    try {
      return {
        virtualTokenReserves: new BN(snap.virtualTokenReserves),
        virtualSolReserves:   new BN(snap.virtualSolReserves),
        realTokenReserves:    new BN(snap.realTokenReserves),
        realSolReserves:      new BN(snap.realSolReserves),
        tokenTotalSupply:     new BN(snap.tokenTotalSupply),
        complete:             snap.complete,
        creator:              null,
        isMayhemMode:         false,
      };
    } catch { return null; }
  }

  private generateReport(): BacktestReport {
    const n         = this.results.length;
    if (n === 0) return emptyReport(this.launched, this.skipped);

    const wins      = this.results.filter(r => r.pnlSOL > 0);
    const losses    = this.results.filter(r => r.pnlSOL <= 0);
    const pnls      = this.results.map(r => r.pnlSOL);
    const totalPnl  = pnls.reduce((a, b) => a + b, 0);
    const avgPnl    = totalPnl / n;
    const variance  = pnls.reduce((a, p) => a + Math.pow(p - avgPnl, 2), 0) / n;
    const stdDev    = Math.sqrt(variance);

    let peak = 0, cumPnl = 0, maxDD = 0;
    for (const pnl of pnls) {
      cumPnl += pnl;
      if (cumPnl > peak) peak = cumPnl;
      const dd = peak - cumPnl;
      if (dd > maxDD) maxDD = dd;
    }

    const reasons: Record<string, number> = {};
    for (const r of this.results) {
      reasons[r.reason] = (reasons[r.reason] ?? 0) + 1;
    }

    const buckets = [5_000, 10_000, 20_000, 35_000, 50_000];
    const mcapAnalysis = buckets.map((cap, i) => {
      const low   = i === 0 ? 0 : buckets[i - 1];
      const trades = this.results.filter(
        r => r.buyMcapUSD >= low && r.buyMcapUSD < cap
      );
      const pnl   = trades.reduce((a, t) => a + t.pnlSOL, 0);
      return { range: `$${low/1000}k–$${cap/1000}k`, trades: trades.length, pnl };
    });

    return {
      totalTrades:    n,
      wins:           wins.length,
      losses:         losses.length,
      winRate:        wins.length / n,
      totalPnlSOL:    totalPnl,
      avgPnlSOL:      avgPnl,
      maxPnlSOL:      Math.max(...pnls),
      minPnlSOL:      Math.min(...pnls),
      stdDev,
      sharpeRatio:    stdDev > 0 ? avgPnl / stdDev : 0,
      maxDrawdownSOL: maxDD,
      avgHoldMs:      this.results.reduce((a, r) => a + r.holdMs, 0) / n,
      sellReasons:    reasons,
      mcapAnalysis,
      launched:       this.launched,
      skipped:        this.skipped,
    };
  }
}

// ─── Types für Report ─────────────────────────────────────────────────────────

interface BacktestReport {
  totalTrades:    number;
  wins:           number;
  losses:         number;
  winRate:        number;
  totalPnlSOL:    number;
  avgPnlSOL:      number;
  maxPnlSOL:      number;
  minPnlSOL:      number;
  stdDev:         number;
  sharpeRatio:    number;
  maxDrawdownSOL: number;
  avgHoldMs:      number;
  sellReasons:    Record<string, number>;
  mcapAnalysis:   { range: string; trades: number; pnl: number }[];
  launched:       number;
  skipped:        number;
}

interface SweepResult {
  config: BacktestConfig;
  report: BacktestReport;
}

function emptyReport(launched: number, skipped: number): BacktestReport {
  return {
    totalTrades: 0, wins: 0, losses: 0, winRate: 0,
    totalPnlSOL: 0, avgPnlSOL: 0, maxPnlSOL: 0, minPnlSOL: 0,
    stdDev: 0, sharpeRatio: 0, maxDrawdownSOL: 0, avgHoldMs: 0,
    sellReasons: {}, mcapAnalysis: [], launched, skipped,
  };
}

// ─── gRPC Recorder ───────────────────────────────────────────────────────────

export async function recordToJSONL(
  grpcEndpoint: string,
  grpcToken:    string,
  pumpProgram:  string,
  outputFile:   string
) {
  const Client = (await import("@triton-one/yellowstone-grpc")).default;
  const { CommitmentLevel } = await import("@triton-one/yellowstone-grpc");

  const client  = new Client(grpcEndpoint, grpcToken, {});
  const stream  = await client.subscribe();
  const out     = fs.createWriteStream(outputFile, { flags: "a" });

  const seenCurves = new Set<string>();

  stream.write({
    accounts: {
      pump: {
        account: [],
        owner:   [pumpProgram],
        filters: [],
      },
    },
    slots: {}, transactions: {}, transactionsStatus: {},
    entry: {}, blocks: {}, blocksMeta: {},
    commitment: CommitmentLevel.PROCESSED,
    accountsDataSlice: [],
    ping: undefined,
  });

  console.log(`[Recorder] Schreibe nach ${outputFile}...`);

  stream.on("data", (data) => {
    if (!data?.account?.account) return;

    const pubkeyBytes = data.account.account.pubkey as Uint8Array;
    if (!pubkeyBytes) return;

    const { PublicKey } = require("@solana/web3.js");
    const addr    = new PublicKey(pubkeyBytes).toBase58();
    const rawData = data.account.account.data as Uint8Array;
    if (!rawData?.length) return;

    const state = decodeCurveState(Buffer.from(rawData), true);
    if (!state) return;

    const isLaunch = !seenCurves.has(addr);
    seenCurves.add(addr);

    const snap: CurveSnapshot = {
      ts:                    Date.now(),
      mint:                  addr,
      virtualTokenReserves:  state.virtualTokenReserves.toString(),
      virtualSolReserves:    state.virtualSolReserves.toString(),
      realTokenReserves:     state.realTokenReserves.toString(),
      realSolReserves:       state.realSolReserves.toString(),
      tokenTotalSupply:      state.tokenTotalSupply.toString(),
      complete:              state.complete,
      isLaunch,
    };

    out.write(JSON.stringify(snap) + "\n");
  });

  stream.on("error", (err) => {
    console.error("[Recorder] Error:", err.message);
    out.close();
  });
}

// ─── Entry Points ─────────────────────────────────────────────────────────────

async function runBacktest() {
  const args = process.argv.slice(2);
  const mode = args[0];

  if (mode === "record") {
    await recordToJSONL(
      process.env.GRPC_ENDPOINT!,
      process.env.GRPC_TOKEN!,
      process.env.PUMP_PROGRAM_ID ?? "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
      args[1] ?? "snapshots.jsonl"
    );
    return;
  }

  if (mode === "sweep") {
    const tester = new Backtester();
    const grid: Partial<BacktestConfig>[] = [];

    for (const tp of [2.0, 2.5, 3.0, 4.0]) {
      for (const sl of [0.4, 0.5, 0.6]) {
        for (const trail of [2000, 3000, 4000]) {
          grid.push({ TP_MULTIPLE: tp, SL_MULTIPLE: sl, TRAILING_STOP_BPS: trail });
        }
      }
    }

    const results = await tester.sweep(args[1] ?? "snapshots.jsonl", grid);

    console.log("\n=== TOP 5 KONFIGURATIONEN (nach Sharpe) ===");
    results.slice(0, 5).forEach((r, i) => {
      console.log(`\n#${i + 1}:`);
      console.log(`  TP=${r.config.TP_MULTIPLE} SL=${r.config.SL_MULTIPLE} Trail=${r.config.TRAILING_STOP_BPS}bps`);
      console.log(`  Trades=${r.report.totalTrades} WinRate=${(r.report.winRate * 100).toFixed(1)}%`);
      console.log(`  PnL=${r.report.totalPnlSOL.toFixed(4)} SOL Sharpe=${r.report.sharpeRatio.toFixed(3)}`);
      console.log(`  MaxDD=${r.report.maxDrawdownSOL.toFixed(4)} SOL`);
      console.log(`  Sell-Reasons:`, r.report.sellReasons);
    });

    fs.writeFileSync("sweep-results.json", JSON.stringify(results, null, 2));
    console.log("\n[Sweep] Vollständige Ergebnisse → sweep-results.json");
    return;
  }

  const tester = new Backtester();
  const report = await tester.run(args[0] ?? "snapshots.jsonl");

  console.log("\n=== BACKTEST REPORT ===");
  console.log(`Launches erkannt:  ${report.launched}`);
  console.log(`Gefiltert/Skipped: ${report.skipped}`);
  console.log(`Trades gesamt:     ${report.totalTrades}`);
  console.log(`Win-Rate:          ${(report.winRate * 100).toFixed(1)}%`);
  console.log(`Total PnL:         ${report.totalPnlSOL.toFixed(4)} SOL`);
  console.log(`Avg PnL/Trade:     ${report.avgPnlSOL.toFixed(4)} SOL`);
  console.log(`Max PnL:           ${report.maxPnlSOL.toFixed(4)} SOL`);
  console.log(`Min PnL:           ${report.minPnlSOL.toFixed(4)} SOL`);
  console.log(`Sharpe Ratio:      ${report.sharpeRatio.toFixed(3)}`);
  console.log(`Max Drawdown:      ${report.maxDrawdownSOL.toFixed(4)} SOL`);
  console.log(`Avg Hold:          ${(report.avgHoldMs / 1000).toFixed(0)}s`);
  console.log(`Sell-Reasons:`, report.sellReasons);
  console.log(`\nMCap-Analyse (wo wird verdient?):`);
  for (const b of report.mcapAnalysis) {
    console.log(`  ${b.range}: ${b.trades} Trades, PnL=${b.pnl.toFixed(4)} SOL`);
  }

  fs.writeFileSync("backtest-report.json", JSON.stringify(report, null, 2));
  console.log("\n[Backtest] Report → backtest-report.json");
}

runBacktest().catch(console.error);
