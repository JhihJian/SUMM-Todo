use std::process;

use todo::cli::{Args, Command};
use todo::commands;
use todo::db::Database;
use todo::error::TodoError;
use todo::output::{output_error, Output};

fn main() {
    let args: Args = argh::from_env();
    let output = Output::new(args.pretty, args.toon, args.json);

    let result = run(args.command, &output);

    match result {
        Ok(text) => {
            println!("{}", text);
            process::exit(0);
        }
        Err(err) => {
            eprintln!("{}", output_error(&err));
            process::exit(err.exit_code());
        }
    }
}

fn run(command: Command, output: &Output) -> Result<String, TodoError> {
    let db = Database::open()?;

    match command {
        Command::Add(args) => commands::add::execute(&db, args, output),
        Command::Next(args) => commands::next::execute(&db, args, output),
        Command::Start(args) => commands::start::execute(&db, args, output),
        Command::Done(args) => commands::done::execute(&db, args, output),
        Command::Block(args) => commands::block::execute(&db, args, output),
        Command::Resume(args) => commands::resume::execute(&db, args, output),
        Command::Cancel(args) => commands::cancel::execute(&db, args, output),
        Command::List(args) => commands::list::execute(&db, args, output),
        Command::Show(args) => commands::show::execute(&db, args, output),
        Command::Log(args) => commands::log::execute(&db, args, output),
        Command::Stats(args) => commands::stats::execute(&db, args, output),
        Command::Import(args) => commands::import::execute(&db, args, output),
        Command::Export(args) => commands::export::execute(&db, args, output),
        Command::Edit(args) => commands::edit::execute(&db, args, output),
        Command::Undo(args) => commands::undo::execute(&db, args, output),
        Command::Abandon(args) => commands::abandon::execute(&db, args, output),
        Command::Search(args) => commands::search::execute(&db, args, output),
        Command::Project(args) => commands::project::execute(&db, args.command, output),
        Command::Sync(args) => commands::sync::execute(&db, args, output),
    }
}
