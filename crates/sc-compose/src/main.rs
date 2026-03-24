use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "sc-compose")]
#[command(about = "Render a template string using JSON data")]
struct Args {
    /// Inline template text.
    #[arg(long)]
    template: String,
    /// JSON object used as the render context.
    #[arg(long, default_value = "{}")]
    json: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let json: serde_json::Value = serde_json::from_str(&args.json)?;
    let rendered = sc_composer::render_template(&args.template, json)?;
    println!("{rendered}");
    Ok(())
}
