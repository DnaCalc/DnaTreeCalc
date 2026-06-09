use dnatreecalc_skin_framework::{
    InitialNodeContentProjection, TemplateManifestProjection, TemplateProjection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuiltInTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

const BUILT_IN_TEMPLATES: &[BuiltInTemplate] = &[
    BuiltInTemplate {
        id: "starter",
        name: "Starter Formula",
        description: "Creates a formula node with simple starter content.",
        content: "=1+1",
    },
    BuiltInTemplate {
        id: "input-zero",
        name: "Input Zero",
        description: "Creates a constant input node initialized to zero.",
        content: "0",
    },
];

pub(crate) fn built_in_template_initial_content(template_id: &str) -> Option<&'static str> {
    BUILT_IN_TEMPLATES
        .iter()
        .find(|template| template.id == template_id)
        .map(|template| template.content)
}

pub(crate) fn built_in_template_manifest_projection() -> TemplateManifestProjection {
    TemplateManifestProjection {
        entries: BUILT_IN_TEMPLATES
            .iter()
            .map(|template| TemplateProjection {
                template_id: template.id.to_string(),
                name: template.name.to_string(),
                description: Some(template.description.to_string()),
                initial: InitialNodeContentProjection::TemplateBound {
                    template_id: template.id.to_string(),
                },
                preview_content: Some(template.content.to_string()),
                built_in: true,
            })
            .collect(),
    }
}
