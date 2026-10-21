---- MODULE Freshness_TTrace_1792620789 ----
EXTENDS Sequences, TLCExt, Toolbox, Freshness, Naturals, TLC

_expression ==
    LET Freshness_TEExpression == INSTANCE Freshness_TEExpression
    IN Freshness_TEExpression!expression
----

_trace ==
    LET Freshness_TETrace == INSTANCE Freshness_TETrace
    IN Freshness_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        recs = ({[before |-> 0, after |-> 0, pass |-> TRUE, stable |-> FALSE]})
        /\
        run = ([active |-> FALSE, before |-> 0, edits |-> 0])
        /\
        wt = (0)
    )
----

_init ==
    /\ run = _TETrace[1].run
    /\ recs = _TETrace[1].recs
    /\ wt = _TETrace[1].wt
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ run  = _TETrace[i].run
        /\ run' = _TETrace[j].run
        /\ recs  = _TETrace[i].recs
        /\ recs' = _TETrace[j].recs
        /\ wt  = _TETrace[i].wt
        /\ wt' = _TETrace[j].wt

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("Freshness_TTrace_1792620789.json", _TETrace)

=============================================================================

 Note that you can extract this module `Freshness_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `Freshness_TEExpression.tla` file takes precedence 
  over the module `Freshness_TEExpression` below).

---- MODULE Freshness_TEExpression ----
EXTENDS Sequences, TLCExt, Toolbox, Freshness, Naturals, TLC

expression == 
    [
        \* To hide variables of the `Freshness` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        run |-> run
        ,recs |-> recs
        ,wt |-> wt
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_runUnchanged |-> run = run'
        
        \* Format the `run` variable as Json value.
        \* ,_runJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(run)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_runModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].run # _TETrace[s-1].run
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE Freshness_TETrace ----
\*EXTENDS IOUtils, Freshness, TLC
\*
\*trace == IODeserialize("Freshness_TTrace_1792620789.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE Freshness_TETrace ----
EXTENDS Freshness, TLC

trace == 
    <<
    ([recs |-> {},run |-> [active |-> FALSE, before |-> 0, edits |-> 0],wt |-> 0]),
    ([recs |-> {},run |-> [active |-> TRUE, before |-> 0, edits |-> 0],wt |-> 0]),
    ([recs |-> {},run |-> [active |-> TRUE, before |-> 0, edits |-> 1],wt |-> 1]),
    ([recs |-> {},run |-> [active |-> TRUE, before |-> 0, edits |-> 2],wt |-> 0]),
    ([recs |-> {[before |-> 0, after |-> 0, pass |-> TRUE, stable |-> FALSE]},run |-> [active |-> FALSE, before |-> 0, edits |-> 0],wt |-> 0])
    >>
----


=============================================================================

---- CONFIG Freshness_TTrace_1792620789 ----
CONSTANTS
    Trees = { 0 , 1 , 2 }
    Rule = "strict"
    MaxEditsPerRun = 2
    MaxRuns = 2

INVARIANT
    _inv

CHECK_DEADLOCK
    \* CHECK_DEADLOCK off because of PROPERTY or INVARIANT above.
    FALSE

INIT
    _init

NEXT
    _next

CONSTANT
    _TETrace <- _trace

ALIAS
    _expression
=============================================================================
\* Generated on Wed Oct 21 22:13:10 UTC 2026