*Posted by Codex agent on behalf of @govert*

# OxVba handoff: missing Application member should diagnose

## DnaOneCalc symptom

DnaOneCalc's first WS-15 host-root slice injects a deliberately narrow
`Application` object for OxVba projects. The only admitted member in the first
slice is:

- `Application.Version`

A local negative probe showed that an unsupported member access can currently
invoke successfully instead of producing a structured diagnostic:

```vb
Public Function MissingApplicationMember() As String
MissingApplicationMember = Application.DoesNotExist
End Function
```

That makes it unsafe for DnaOneCalc to close the `Application.Version` bead as
"no broader Excel Application surface implied" until OxVba reports the missing
member explicitly.

## Observed probe

From `C:\Work\DnaCalc\DnaOneCalc` on 2026-05-30:

1. Build a hosted in-memory source project.
2. Prepend the synthetic DnaOneCalc `Application` class module:

```vb
Attribute VB_Name = "Application"
Attribute VB_PredeclaredId = True
Public Property Get Version() As String
Version = "0.1.0-test"
End Property
```

3. Add `MissingApplicationMember` above.
4. Load through `oxvba_host::VbaHost`.
5. Prepare and invoke `Module1.MissingApplicationMember`.

Expected: load, prepare, or invocation fails with a diagnostic naming
`Application.DoesNotExist`.

Actual: invocation returns success.

## Likely upstream root cause

The failure appears to be in OxVba's member resolution or runtime dispatch for a
predeclared class instance. A member not present on the class module should not be
treated as an invokable/readable member.

This is an OxVba semantic/diagnostic issue. DnaOneCalc should not add a host-side
allowlist or post-invocation check to fabricate missing-member diagnostics.

## Proposed upstream surface change

OxVba should return a structured diagnostic for missing members on predeclared
class objects during the earliest correct phase:

- bind/prepare if member resolution is static enough,
- otherwise invocation if the current host object dispatch remains dynamic.

The diagnostic should include:

- object/class name: `Application`
- member name: `DoesNotExist`
- phase: bind, prepare, or invoke
- source span when available

## DnaOneCalc follow-up

After the OxVba fix lands, DnaOneCalc should add the negative regression under
`services::vba_host` and close the WS-15 `Application.Version` bead only when the
positive `Application.Version` probe and the missing-member diagnostic probe both
pass.
