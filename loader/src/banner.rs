//! Version banner (TOTORONG-style phase banner).

use log::info;

pub fn print(version: &str) {
    info!("==================================================");
    info!(" nv-uefi-unlocker v{version} (built {})", env!("BUILD_TS"));
    #[cfg(feature = "tx")]
    {
        info!(" ** TX BUILD: CONTAINS GPU-WRITE PATHS **");
        // Armed mode flips the confirm semantics: a countdown auto-runs and
        // ANY key aborts, so the space/enter/30s-timeout prompt would
        // describe a flow that cannot happen - print it only unarmed.
        let armed = crate::config::armed().is_some();
        // Quiet mode (issue #38): a compact banner with PROMPT-tagged lines
        // so the console whitelist keeps the actionable text on slow GOPs.
        // The full banner stays byte-identical to previous releases.
        if crate::logging::is_quiet() {
            if !armed {
                info!("PROMPT: PRESS SPACE or ENTER to CONTINUE; other key / 30s no-key = ABORT (unlocks NOTHING)");
            }
            info!(
                "PROMPT: armed \\i_know_what_i_am_doing.txt = 5s auto-run, any key aborts; multi-GPU: all supported cards in order, \\tx-select-bdf.txt (bb:dd.f) pins one; failure = emergency FLR (next card still runs)"
            );
        } else {
            info!(" transaction: claim->gfw->fwsec->booter->selectors->flr");
            if !armed {
                // Triple prompt: UEFI console text is fly-sized on high-DPI panels.
                info!("======== PRESS SPACE or ENTER to CONTINUE ========");
                info!("======== PRESS SPACE or ENTER to CONTINUE ========");
                info!("======== PRESS SPACE or ENTER to CONTINUE ========");
                info!("  other key or 30s no-key = ABORT; unlocks NOTHING");
                info!("  other key or 30s no-key = ABORT; unlocks NOTHING");
                info!("  other key or 30s no-key = ABORT; unlocks NOTHING");
            }
            info!(" armed: \\i_know_what_i_am_doing.txt = 5s auto-run, any key aborts");
            info!(" multi-GPU: no marker = every supported card in order;");
            info!("            \\tx-select-bdf.txt (bb:dd.f) pins exactly one");
            info!(" failure = emergency FLR (per card; the next card still runs)");
        }
    }
    #[cfg(all(feature = "diag", not(feature = "tx")))]
    {
        info!(" ** DIAG BUILD: SURGERY ONLY - NO GPU-WRITE PATHS COMPILED **");
        info!(" \\tx-agitate-only.txt = full-surgery zero-GPU-write control;");
        info!(" \\tx-ebs-watch.txt = ExitBootServices observer; transaction code is not compiled in");
    }
    #[cfg(not(any(feature = "tx", feature = "diag")))]
    {
        info!(" read-only build: enumerate + sample + chain-load");
        info!(" console output only; no GPU writes by design");
    }
    info!("==================================================");
    info!("PHASE: CONFIRM");
}
