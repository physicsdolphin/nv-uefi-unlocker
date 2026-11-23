# Phase 2 corrected: tx_one_card is the LAST item in main.rs; the function
# tail is `... CardOutcome::Attempted\n                }\n            }\n        }\n    }\n}`.
# Gate the post-agitate remainder in a #[cfg(feature = "tx")] block and add
# the diag fallback arm right after the agitate branch.
import io

p = r'loader/src/main.rs'
s = io.open(p, encoding='utf-8').read()

old = r'''        tx::handoff_out(&pci);
        return CardOutcome::Attempted;
    }

    // BridgeMaster guard slot: run_transaction fills it mid-transaction (after'''
new = r'''        tx::handoff_out(&pci);
        return CardOutcome::Attempted;
    }

    #[cfg(not(feature = "tx"))]
    {
        // DIAG build: the payload/transaction machinery is not compiled in.
        // The agitate control above was the only GPU-touching branch this
        // binary carries, and it writes nothing to BAR0 by contract.
        info!("DIAG BUILD: transaction code is not compiled in - no unlock, no replay; the agitate control above was the only action this binary can take");
        tx::handoff_out(&pci);
        return CardOutcome::Attempted;
    }

    #[cfg(feature = "tx")]
    {
    // BridgeMaster guard slot: run_transaction fills it mid-transaction (after'''
assert s.count(old) == 1, "branch head: %d" % s.count(old)
s = s.replace(old, new)

old_tail = r'''                    tx::handoff_out(&pci);
                    CardOutcome::Attempted
                }
            }
        }
    }
}
'''
assert s.count(old_tail) == 1, "tail: %d" % s.count(old_tail)
new_tail = r'''                    tx::handoff_out(&pci);
                    CardOutcome::Attempted
                }
            }
        }
    }
    }
}
'''
s = s.replace(old_tail, new_tail)
io.open(p, 'w', encoding='utf-8', newline='').write(s)
print("patched tx_one_card")
