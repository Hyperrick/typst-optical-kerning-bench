mod commands;
mod corpus;
mod data;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "optikern")]
#[command(about = "Reproducible optical kerning benchmark suite for Typst")]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Download pinned Google Fonts into corpus/fonts.
    FetchFonts {
        #[arg(long)]
        force: bool,
    },
    /// Evaluate all configured fonts and critical pairs.
    Bench,
    /// Generate and compile Typst comparison sheets.
    RenderTypst {
        #[arg(long)]
        no_compile: bool,
    },
    /// Generate InDesign ExtendScript for Metrics and Optical PDFs.
    RenderIndesign {
        #[arg(long)]
        run: bool,
    },
    /// Render PDFs to PNGs and compute simple black-pixel bounding boxes.
    EvalPdf {
        #[arg(long, default_value = "renders")]
        input: PathBuf,
    },
    /// Build HTML and Typst/PDF reports from metrics/bench.json.
    Report {
        #[arg(long)]
        no_compile: bool,
    },
    /// Build a compact A3 visual contact sheet from metrics/bench.json.
    ContactSheet {
        #[arg(long)]
        no_compile: bool,
    },
    /// Compare InDesign Optical against Typst Metric and guarded optical.
    TriadCompare {
        #[arg(long)]
        run_indesign: bool,
        #[arg(long)]
        no_compile: bool,
    },
    /// Print guarded optical deltas for one shaped sample.
    SampleDeltas {
        #[arg(long)]
        font_id: String,
        #[arg(long)]
        text: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        ligatures: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::FetchFonts { force } => commands::fetch_fonts::run(&cli.root, force),
        Command::Bench => commands::bench::run(&cli.root),
        Command::RenderTypst { no_compile } => commands::render_typst::run(&cli.root, !no_compile),
        Command::RenderIndesign { run } => commands::render_indesign::run(&cli.root, run),
        Command::EvalPdf { input } => commands::eval_pdf::run(&cli.root, &input),
        Command::Report { no_compile } => commands::report::run(&cli.root, !no_compile),
        Command::ContactSheet { no_compile } => {
            commands::contact_sheet::run(&cli.root, !no_compile)
        }
        Command::TriadCompare {
            run_indesign,
            no_compile,
        } => commands::triad_compare::run(&cli.root, run_indesign, !no_compile),
        Command::SampleDeltas {
            font_id,
            text,
            ligatures,
        } => commands::sample_deltas::run(&cli.root, &font_id, &text, ligatures),
    }
}
