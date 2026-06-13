Status: Open
Target: DnaOneCalc
Ask: Check OneCalc error-value display and route CalcValue visible text through OxFml's display formatter instead of enum/debug text.
Context: While fixing DnaTreeCalc dtc-366.1, the user reported seeing the same bare "Value" display in DNA OneCalc when the actual worksheet error was #VALUE!. TreeCalc had a local CalcValue display fallback that formatted WorksheetErrorCode::Value as "Value"; that fallback is now deleted and TreeCalc calls oxfml_core::format::render_visible_value_text.
Evidence: In DnaTreeCalc, a real error-value repro via =ABS("abc") now projects NodeValueProjection::Error("#VALUE!"). The direct WorksheetErrorCode projection test covers #NULL!, #DIV/0!, #VALUE!, #REF!, #NAME?, #NUM!, #N/A, #BUSY!, #GETTING_DATA, #SPILL!, #CALC!, #FIELD!, #BLOCKED!, and #CONNECT!.
