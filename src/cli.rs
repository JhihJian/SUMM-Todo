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
    Export(ExportArgs),
    Edit(EditArgs),
    Undo(UndoArgs),
    Abandon(AbandonArgs),
    Search(SearchArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "add")]
/// Create a new task
pub struct AddArgs {
    /// task title
    #[argh(positional)]
    pub title: String,

    /// priority level
    #[argh(option, short = 'P')]
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

    /// detailed description (supports multi-line)
    #[argh(option, short = 'd', long = "description")]
    pub description: Option<String>,

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
    #[argh(option, short = 'P')]
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
    #[argh(option, short = 'a')]
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
    #[argh(option, short = 'r')]
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
    #[argh(option, short = 'P')]
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

    /// show only overdue tasks
    #[argh(switch)]
    pub overdue: bool,

    /// show all tasks including done and cancelled
    #[argh(switch)]
    pub all: bool,
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

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "export")]
/// Export tasks to JSON file
pub struct ExportArgs {
    /// output file path (default: stdout)
    #[argh(option, short = 'f')]
    pub file: Option<String>,

    /// filter by status (repeatable)
    #[argh(option, short = 's')]
    pub status: Vec<String>,

    /// filter by tag (repeatable)
    #[argh(option, short = 't')]
    pub tag: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "edit")]
/// Edit task properties
pub struct EditArgs {
    /// task id
    #[argh(positional)]
    pub id: String,

    /// new title
    #[argh(option)]
    pub title: Option<String>,

    /// new priority
    #[argh(option, short = 'P')]
    pub priority: Option<String>,

    /// add tag (prefix with +) or remove tag (prefix with -)
    #[argh(option, short = 't')]
    pub tag: Vec<String>,

    /// new due date
    #[argh(option)]
    pub due: Option<String>,

    /// new description
    #[argh(option, short = 'd', long = "description")]
    pub description: Option<String>,

    /// clear the description
    #[argh(switch, long = "clear-content")]
    pub clear_content: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "undo")]
/// Undo a completed task (done -> in_progress)
pub struct UndoArgs {
    /// task id
    #[argh(positional)]
    pub id: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "abandon")]
/// Release a task back to pending (in_progress -> pending)
pub struct AbandonArgs {
    /// task id
    #[argh(positional)]
    pub id: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "search")]
/// Search tasks by title or content
pub struct SearchArgs {
    /// search query
    #[argh(positional)]
    pub query: String,

    /// filter by tag
    #[argh(option, short = 't')]
    pub tag: Option<String>,

    /// filter by status
    #[argh(option, short = 's')]
    pub status: Option<String>,

    /// use regex matching
    #[argh(switch)]
    pub regex: bool,
}
