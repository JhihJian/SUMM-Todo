use argh::FromArgs;

#[derive(FromArgs)]
/// Todo - Human-Agent Task Coordination Protocol
pub struct Args {
    #[argh(subcommand)]
    pub command: Command,

    /// output in human-readable format
    #[argh(switch, short = 'p')]
    pub pretty: bool,

    /// output in TOON format (token-efficient for LLMs, default)
    #[argh(switch)]
    pub toon: bool,

    /// output in JSON format (for backwards compatibility)
    #[argh(switch)]
    pub json: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Command {
    Add(AddArgs),
    Next(NextArgs),
    Start(StartArgs),
    Done(DoneArgs),
    Block(BlockArgs),
    Resume(ResumeArgs),
    Cancel(CancelArgs),
    List(ListArgs),
    Show(ShowArgs),
    Log(LogArgs),
    Stats(StatsArgs),
    Import(ImportArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "add")]
/// Create a new task
pub struct AddArgs {
    /// task title
    #[argh(positional)]
    pub title: String,

    /// priority level
    #[argh(option, short = 'r')]
    pub pri: Option<String>,

    /// tag (repeatable)
    #[argh(option, short = 't')]
    pub tag: Vec<String>,

    /// parent task id
    #[argh(option)]
    pub parent: Option<String>,

    /// due date
    #[argh(option)]
    pub due: Option<String>,

    /// creator name
    #[argh(option)]
    pub creator: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "next")]
/// Claim the next pending task
pub struct NextArgs {
    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// filter by priority
    #[argh(option, short = 'r')]
    pub pri: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "start")]
/// Start a specific task
pub struct StartArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// assignee name
    #[argh(option)]
    pub assignee: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "done")]
/// Complete a task
pub struct DoneArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// result summary
    #[argh(option, short = 'm')]
    pub result: String,

    /// artifact reference (repeatable)
    #[argh(option)]
    pub artifact: Vec<String>,

    /// execution log entry
    #[argh(option)]
    pub log: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "block")]
/// Block a task
pub struct BlockArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// reason for blocking
    #[argh(option)]
    pub reason: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "resume")]
/// Resume a blocked task
pub struct ResumeArgs {
    /// task id
    #[argh(positional)]
    pub id: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "cancel")]
/// Cancel a task
pub struct CancelArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// reason for cancellation
    #[argh(option)]
    pub reason: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
/// List tasks with filters
pub struct ListArgs {
    /// filter by status (repeatable)
    #[argh(option, short = 's')]
    pub status: Vec<String>,

    /// filter by tag (repeatable)
    #[argh(option, short = 't')]
    pub tag: Vec<String>,

    /// filter by priority
    #[argh(option, short = 'r')]
    pub pri: Option<String>,

    /// filter by parent task id
    #[argh(option)]
    pub parent: Option<String>,

    /// filter by creator
    #[argh(option)]
    pub creator: Option<String>,

    /// filter tasks since date
    #[argh(option)]
    pub since: Option<String>,

    /// maximum number of results
    #[argh(option)]
    pub limit: Option<i64>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "show")]
/// Show task details
pub struct ShowArgs {
    /// task id
    #[argh(positional)]
    pub id: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "log")]
/// View execution log
pub struct LogArgs {
    /// show only today's entries
    #[argh(switch)]
    pub today: bool,

    /// filter entries since date
    #[argh(option)]
    pub since: Option<String>,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "stats")]
/// Show task statistics
pub struct StatsArgs {
    /// filter stats since date
    #[argh(option)]
    pub since: Option<String>,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "import")]
/// Bulk import tasks from JSON file
pub struct ImportArgs {
    /// path to JSON file (use "-" for stdin)
    #[argh(option, short = 'f')]
    pub file: String,
}
