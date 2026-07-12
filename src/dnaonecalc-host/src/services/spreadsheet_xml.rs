use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use roxmltree::{Document, Node};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct SpreadsheetXmlCellExtraction {
    pub workbook_path: String,
    pub locator: String,
    pub worksheet_name: String,
    pub workbook_format_profile_hint: String,
    pub formula_text: Option<String>,
    pub entered_cell_text: String,
    pub data_type: Option<String>,
    pub style_id: Option<String>,
    pub style_hierarchy: Vec<String>,
    pub number_format_code: Option<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    pub conditional_formats: Vec<ConditionalFormatRule>,
    pub date1904: Option<bool>,
    pub observation_scope: VerificationObservationScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct ConditionalFormatRule {
    pub range: String,
    pub formula: Option<String>,
    pub value1: Option<String>,
    pub value2: Option<String>,
    pub operator: Option<String>,
    pub rule_kind: Option<String>,
    pub interior_color: Option<String>,
    pub font_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct VerificationObservationScope {
    pub oxfml_required_scope: Vec<String>,
    pub oxxlplay_required_surfaces: Vec<String>,
    pub oxreplay_required_views: Vec<String>,
}

#[derive(Debug, Clone)]
struct StyleInfo {
    parent_style_id: Option<String>,
    style_hierarchy: Vec<String>,
    number_format_code: Option<String>,
    font_color: Option<String>,
    fill_color: Option<String>,
}

pub fn extract_cell_from_spreadsheet_xml(
    workbook_path: impl AsRef<Path>,
    locator: &str,
) -> Result<SpreadsheetXmlCellExtraction, String> {
    let workbook_path = workbook_path.as_ref();
    let xml = fs::read_to_string(workbook_path).map_err(|error| {
        format!(
            "failed to read SpreadsheetML workbook `{}`: {error}",
            workbook_path.display()
        )
    })?;
    let document = Document::parse(&xml).map_err(|error| {
        format!(
            "failed to parse SpreadsheetML workbook `{}`: {error}",
            workbook_path.display()
        )
    })?;

    let (worksheet_name, cell_ref) = split_locator(locator)?;
    let worksheet = find_worksheet(&document, worksheet_name)?;
    let styles = collect_styles(&document);
    let cell = find_cell(worksheet, cell_ref)?;

    let style_id = attr_ns(cell, "StyleID").map(ToOwned::to_owned);
    let style_info = style_id
        .as_deref()
        .and_then(|style_id| styles.get(style_id))
        .cloned();
    let formula_text = attr_ns(cell, "Formula").map(ToOwned::to_owned);
    let data_node = cell
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "Data");
    let data_type = data_node
        .and_then(|node| attr_ns(node, "Type"))
        .map(ToOwned::to_owned);
    let data_text = data_node
        .and_then(|node| node.text())
        .unwrap_or("")
        .to_string();
    let entered_cell_text = formula_text
        .clone()
        .unwrap_or_else(|| rebuild_entered_text(&data_text, data_type.as_deref()));
    let conditional_formats = collect_conditional_formats(worksheet, cell_ref);

    Ok(SpreadsheetXmlCellExtraction {
        workbook_path: workbook_path.to_string_lossy().replace('\\', "/"),
        locator: locator.to_string(),
        worksheet_name: worksheet_name.to_string(),
        workbook_format_profile_hint: "excel-spreadsheetml-2003-default".to_string(),
        formula_text,
        entered_cell_text,
        data_type,
        style_id,
        style_hierarchy: style_info
            .as_ref()
            .map(|style| style.style_hierarchy.clone())
            .unwrap_or_default(),
        number_format_code: style_info
            .as_ref()
            .and_then(|style| style.number_format_code.clone()),
        font_color: style_info
            .as_ref()
            .and_then(|style| style.font_color.clone()),
        fill_color: style_info
            .as_ref()
            .and_then(|style| style.fill_color.clone()),
        conditional_formats,
        date1904: detect_date1904(&document),
        observation_scope: VerificationObservationScope {
            oxfml_required_scope: vec![
                "entered_cell_text".to_string(),
                "format_profile".to_string(),
                "locale_format_context".to_string(),
                "date1904".to_string(),
                "number_format_code".to_string(),
                "style_id".to_string(),
                "style_hierarchy".to_string(),
                "format_dependency_facts".to_string(),
                "format_delta".to_string(),
                "display_delta".to_string(),
                "presentation_hint".to_string(),
                "returned_value_surface".to_string(),
                "font_color".to_string(),
                "fill_color".to_string(),
                "conditional_formatting_rules".to_string(),
                "conditional_formatting_target_ranges".to_string(),
                "conditional_formatting_rule_kind".to_string(),
                "conditional_formatting_operator".to_string(),
                "conditional_formatting_thresholds".to_string(),
                "conditional_formatting_effective_display".to_string(),
            ],
            oxxlplay_required_surfaces: vec![
                "formula_text".to_string(),
                "cell_value".to_string(),
                "effective_display_text".to_string(),
                "number_format_code".to_string(),
                "style_id".to_string(),
                "font_color".to_string(),
                "fill_color".to_string(),
                "conditional_formatting_rules".to_string(),
                "conditional_formatting_effective_style".to_string(),
            ],
            oxreplay_required_views: vec![
                "comparison_value".to_string(),
                "effective_display_text".to_string(),
                "formatting_view".to_string(),
                "conditional_formatting_view".to_string(),
            ],
        },
    })
}

fn split_locator(locator: &str) -> Result<(&str, &str), String> {
    let mut parts = locator.splitn(2, '!');
    let worksheet_name = parts
        .next()
        .ok_or_else(|| format!("invalid locator `{locator}`"))?;
    let cell_ref = parts
        .next()
        .ok_or_else(|| format!("invalid locator `{locator}`; expected Sheet!Cell"))?;
    Ok((worksheet_name, cell_ref))
}

fn find_worksheet<'a>(
    document: &'a Document<'a>,
    worksheet_name: &str,
) -> Result<Node<'a, 'a>, String> {
    document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "Worksheet"
                && attr_ns(*node, "Name") == Some(worksheet_name)
        })
        .ok_or_else(|| format!("worksheet `{worksheet_name}` not found in SpreadsheetML workbook"))
}

fn find_cell<'a>(worksheet: Node<'a, 'a>, target_ref: &str) -> Result<Node<'a, 'a>, String> {
    let (target_col, target_row) = parse_a1_ref(target_ref)?;
    let table = worksheet
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "Table")
        .ok_or_else(|| format!("worksheet is missing a Table for target `{target_ref}`"))?;

    let mut current_row = 0usize;
    for row in table
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "Row")
    {
        if let Some(index) = attr_ns(row, "Index").and_then(|value| value.parse::<usize>().ok()) {
            current_row = index;
        } else {
            current_row += 1;
        }
        if current_row != target_row {
            continue;
        }

        let mut current_col = 0usize;
        for cell in row
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "Cell")
        {
            if let Some(index) =
                attr_ns(cell, "Index").and_then(|value| value.parse::<usize>().ok())
            {
                current_col = index;
            } else {
                current_col += 1;
            }
            if current_col == target_col {
                return Ok(cell);
            }
        }
    }

    Err(format!("cell `{target_ref}` not found in worksheet"))
}

fn collect_styles(document: &Document<'_>) -> BTreeMap<String, StyleInfo> {
    let mut raw_styles = BTreeMap::new();
    for style in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "Style")
    {
        if let Some(style_id) = attr_ns(style, "ID") {
            let number_format_code = style
                .children()
                .find(|node| node.is_element() && node.tag_name().name() == "NumberFormat")
                .and_then(|node| attr_ns(node, "Format"))
                .map(ToOwned::to_owned);
            let font_color = style
                .children()
                .find(|node| node.is_element() && node.tag_name().name() == "Font")
                .and_then(|node| attr_ns(node, "Color"))
                .map(ToOwned::to_owned);
            let fill_color = style
                .children()
                .find(|node| node.is_element() && node.tag_name().name() == "Interior")
                .and_then(|node| attr_ns(node, "Color"))
                .map(ToOwned::to_owned);
            raw_styles.insert(
                style_id.to_string(),
                StyleInfo {
                    parent_style_id: attr_ns(style, "Parent").map(ToOwned::to_owned),
                    style_hierarchy: vec![style_id.to_string()],
                    number_format_code,
                    font_color,
                    fill_color,
                },
            );
        }
    }

    let style_ids = raw_styles.keys().cloned().collect::<Vec<_>>();
    let mut resolved = BTreeMap::new();
    for style_id in style_ids {
        let style = resolve_style_info(&style_id, &raw_styles);
        resolved.insert(style_id, style);
    }
    resolved
}

fn resolve_style_info(style_id: &str, styles: &BTreeMap<String, StyleInfo>) -> StyleInfo {
    let Some(style) = styles.get(style_id) else {
        return StyleInfo {
            parent_style_id: None,
            style_hierarchy: Vec::new(),
            number_format_code: None,
            font_color: None,
            fill_color: None,
        };
    };

    let parent = style
        .parent_style_id
        .as_deref()
        .and_then(|parent_style_id| {
            styles
                .get(parent_style_id)
                .map(|_| resolve_style_info(parent_style_id, styles))
        });

    let mut style_hierarchy = parent
        .as_ref()
        .map(|value| value.style_hierarchy.clone())
        .unwrap_or_default();
    if !style_hierarchy.iter().any(|value| value == style_id) {
        style_hierarchy.push(style_id.to_string());
    }

    StyleInfo {
        parent_style_id: style.parent_style_id.clone(),
        style_hierarchy,
        number_format_code: style.number_format_code.clone().or_else(|| {
            parent
                .as_ref()
                .and_then(|value| value.number_format_code.clone())
        }),
        font_color: style
            .font_color
            .clone()
            .or_else(|| parent.as_ref().and_then(|value| value.font_color.clone())),
        fill_color: style
            .fill_color
            .clone()
            .or_else(|| parent.as_ref().and_then(|value| value.fill_color.clone())),
    }
}

fn collect_conditional_formats(
    worksheet: Node<'_, '_>,
    target_ref: &str,
) -> Vec<ConditionalFormatRule> {
    worksheet
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "ConditionalFormatting")
        .filter_map(|node| {
            let range = attr_ns(node, "Range")?.to_string();
            if !range_contains_ref(&range, target_ref) {
                return None;
            }

            let condition = node
                .descendants()
                .find(|child| child.is_element() && child.tag_name().name() == "Condition");
            let interior_color = node
                .descendants()
                .find(|child| child.is_element() && child.tag_name().name() == "Interior")
                .and_then(|child| attr_ns(child, "Color"))
                .map(ToOwned::to_owned);
            let font_color = node
                .descendants()
                .find(|child| child.is_element() && child.tag_name().name() == "Font")
                .and_then(|child| attr_ns(child, "Color"))
                .map(ToOwned::to_owned);

            Some(ConditionalFormatRule {
                range,
                formula: condition
                    .and_then(|node| attr_ns(node, "Formula"))
                    .map(ToOwned::to_owned),
                value1: condition
                    .and_then(|node| attr_ns(node, "Value1"))
                    .map(ToOwned::to_owned),
                value2: condition
                    .and_then(|node| attr_ns(node, "Value2"))
                    .map(ToOwned::to_owned),
                operator: condition
                    .and_then(|node| attr_ns(node, "Operator"))
                    .map(ToOwned::to_owned),
                rule_kind: condition
                    .and_then(|node| attr_ns(node, "Type"))
                    .map(ToOwned::to_owned),
                interior_color,
                font_color,
            })
        })
        .collect()
}

fn detect_date1904(document: &Document<'_>) -> Option<bool> {
    document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "Date1904")
        .map(|node| node.text().map(|text| text == "1").unwrap_or(true))
}

fn rebuild_entered_text(data_text: &str, data_type: Option<&str>) -> String {
    match data_type {
        Some("String") => data_text.to_string(),
        Some("Number") | Some("Boolean") => data_text.to_string(),
        _ => data_text.to_string(),
    }
}

fn parse_a1_ref(cell_ref: &str) -> Result<(usize, usize), String> {
    let mut letters = String::new();
    let mut digits = String::new();
    for ch in cell_ref.chars() {
        if ch.is_ascii_alphabetic() {
            letters.push(ch.to_ascii_uppercase());
        } else if ch.is_ascii_digit() {
            digits.push(ch);
        }
    }
    if letters.is_empty() || digits.is_empty() {
        return Err(format!("invalid A1 cell reference `{cell_ref}`"));
    }
    let col = letters.chars().fold(0usize, |acc, ch| {
        acc * 26 + (ch as usize - 'A' as usize + 1)
    });
    let row = digits
        .parse::<usize>()
        .map_err(|_| format!("invalid row digits in A1 cell reference `{cell_ref}`"))?;
    Ok((col, row))
}

fn range_contains_ref(range: &str, target_ref: &str) -> bool {
    range.split(',').any(|segment| {
        let segment = segment.trim();
        if let Some((start, end)) = segment.split_once(':') {
            if let (
                Ok((start_col, start_row)),
                Ok((end_col, end_row)),
                Ok((target_col, target_row)),
            ) = (
                parse_a1_ref(start),
                parse_a1_ref(end),
                parse_a1_ref(target_ref),
            ) {
                return target_col >= start_col
                    && target_col <= end_col
                    && target_row >= start_row
                    && target_row <= end_row;
            }
            false
        } else {
            segment.eq_ignore_ascii_case(target_ref)
        }
    })
}

fn attr_ns<'a>(node: Node<'a, 'a>, local_name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.name() == local_name)
        .map(|attribute| attribute.value())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn extracts_formula_style_and_conditional_format_from_spreadsheetml() {
        let temp_path = std::env::temp_dir().join(format!(
            "onecalc-spreadsheetml-{}.xml",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let xml = r##"<?xml version="1.0"?>
<?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:x="urn:schemas-microsoft-com:office:excel">
  <Styles>
    <Style ss:ID="calcBase">
      <NumberFormat ss:Format="$#,##0.00"/>
      <Font ss:Color="#112233"/>
      <Interior ss:Color="#445566"/>
    </Style>
    <Style ss:ID="calc" ss:Parent="calcBase"/>
  </Styles>
  <Worksheet ss:Name="Sheet1">
    <Table>
      <Row>
        <Cell ss:StyleID="calc" ss:Formula="=SUM(1,2,3)"><Data ss:Type="Number">0</Data></Cell>
      </Row>
    </Table>
    <ConditionalFormatting ss:Range="A1">
      <Condition ss:Type="Expression" ss:Formula="=A1>0"/>
      <Font ss:Color="#FF0000"/>
      <Interior ss:Color="#00FF00"/>
    </ConditionalFormatting>
  </Worksheet>
</Workbook>"##;
        fs::write(&temp_path, xml).expect("xml should write");

        let extraction = extract_cell_from_spreadsheet_xml(&temp_path, "Sheet1!A1")
            .expect("extraction should succeed");

        assert_eq!(extraction.formula_text.as_deref(), Some("=SUM(1,2,3)"));
        assert_eq!(extraction.entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(
            extraction.workbook_format_profile_hint,
            "excel-spreadsheetml-2003-default"
        );
        assert_eq!(extraction.number_format_code.as_deref(), Some("$#,##0.00"));
        assert_eq!(extraction.font_color.as_deref(), Some("#112233"));
        assert_eq!(extraction.fill_color.as_deref(), Some("#445566"));
        assert_eq!(
            extraction.style_hierarchy,
            vec!["calcBase".to_string(), "calc".to_string()]
        );
        assert_eq!(extraction.conditional_formats.len(), 1);
        let rule = &extraction.conditional_formats[0];
        assert_eq!(rule.range, "A1");
        assert_eq!(rule.formula.as_deref(), Some("=A1>0"));
        assert_eq!(rule.rule_kind.as_deref(), Some("Expression"));
        assert_eq!(rule.font_color.as_deref(), Some("#FF0000"));
        assert_eq!(rule.interior_color.as_deref(), Some("#00FF00"));

        let _ = fs::remove_file(temp_path);
    }
}
