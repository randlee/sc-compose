use minijinja::{Environment, Error};

/// Render a template string with the provided serializable context.
pub fn render_template<T: serde::Serialize>(template: &str, context: T) -> Result<String, Error> {
    let mut env = Environment::new();
    env.add_template("inline", template)?;
    let template = env.get_template("inline")?;
    template.render(context)
}

#[cfg(test)]
mod tests {
    use super::render_template;

    #[test]
    fn renders_inline_template() {
        let rendered =
            render_template("hello {{ name }}", serde_json::json!({ "name": "world" })).unwrap();
        assert_eq!(rendered, "hello world");
    }
}
