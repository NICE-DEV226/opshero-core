---------------------------- MODULE Freshness ----------------------------
(***************************************************************************)
(* Freshness of evidence (RFC-0002 section 1, RFC-0005 section 7).          *)
(*                                                                         *)
(* A recorder measures the content tree before a command starts and after   *)
(* it finishes. Content can be edited at any time, including while the      *)
(* command runs. The verifier answers PASS for the current content when a   *)
(* passing record is bound to it.                                           *)
(*                                                                         *)
(* Safety property checked (Sound): a PASS accepted for the current content *)
(* is backed by a run during which the content never changed. `stable` is a *)
(* ghost field: the verifier cannot see it, which is the whole problem.     *)
(*                                                                         *)
(* Rule = "allow"  : a record counts when tree_after = current content      *)
(*                   (behaviour of opshero v0.0.1)                          *)
(* Rule = "strict" : additionally tree_before = tree_after                  *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS Trees,            \* finite set of possible content ids
          Rule,             \* "allow" or "strict"
          MaxEditsPerRun,   \* edits that may happen while one command runs
          MaxRuns           \* bound on the number of records

VARIABLES wt,               \* current content of the working tree
          run,              \* [active, before, edits]: the run in progress, if any
          recs              \* set of records

Idle == [active |-> FALSE, before |-> CHOOSE t \in Trees : TRUE, edits |-> 0]

Accept(r) == IF Rule = "allow" THEN TRUE ELSE r.before = r.after

Counts(r) == r.pass /\ r.after = wt /\ Accept(r)

Verdict == IF \E r \in recs : Counts(r) THEN "PASS" ELSE "NOT_PASS"

TypeOK ==
  /\ wt \in Trees
  /\ run.active \in BOOLEAN /\ run.before \in Trees /\ run.edits \in 0..MaxEditsPerRun
  /\ \A r \in recs : r.before \in Trees /\ r.after \in Trees
                     /\ r.pass \in BOOLEAN /\ r.stable \in BOOLEAN

Init == /\ wt \in Trees
        /\ run = Idle
        /\ recs = {}

Edit == /\ wt' \in Trees \ {wt}
        /\ IF run.active
              THEN /\ run.edits < MaxEditsPerRun
                   /\ run' = [run EXCEPT !.edits = @ + 1]
              ELSE run' = run
        /\ UNCHANGED recs

Start == /\ ~run.active
         /\ Cardinality(recs) < MaxRuns
         /\ run' = [active |-> TRUE, before |-> wt, edits |-> 0]
         /\ UNCHANGED <<wt, recs>>

Finish == /\ run.active
          /\ \E p \in BOOLEAN :
               recs' = recs \cup {[before |-> run.before, after |-> wt,
                                   pass |-> p, stable |-> (run.edits = 0)]}
          /\ run' = Idle
          /\ UNCHANGED wt

Next == Edit \/ Start \/ Finish

Spec == Init /\ [][Next]_<<wt, run, recs>>

\* Safety: PASS is never answered on the strength of a run during which the
\* content changed.
Sound == \A r \in recs : Counts(r) => r.stable

\* Usefulness: a stable passing run on the current content is always accepted.
Complete == \A r \in recs : (r.pass /\ r.stable /\ r.after = wt) => Verdict = "PASS"
=============================================================================
