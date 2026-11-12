//! Help texts: cmux's (rc §9, `CLI/cmux.swift`, GPL) trimmed to what is ported (plan D10).

/// (command, usage line, help text). Groups are cmux's task groups.
pub(crate) const GROUPS: &[(&str, &[&str])] = &[
    (
        "Start & Resume",
        &[
            "new-workspace",
            "ssh",
            "claude-teams",
            "vault",
            "local-tmux",
        ],
    ),
    (
        "Navigate & Arrange",
        &[
            "new-split",
            "browser",
            "focus-pane",
            "focus-panel",
            "select-workspace",
            "workspace-group",
        ],
    ),
    (
        "Inspect",
        &[
            "list-workspaces",
            "list-panes",
            "list-pane-surfaces",
            "current-workspace",
        ],
    ),
    (
        "Automation",
        &[
            "send",
            "send-key",
            "notify",
            "dismiss-notification",
            "agent",
            "automation",
            "hooks",
        ],
    ),
    ("Diagnostics / Advanced", &["ping", "version"]),
];

pub(crate) fn usage_line(cmd: &str) -> Option<&'static str> {
    Some(match cmd {
        "new-workspace" => {
            "new-workspace [--name <title>] [--cwd <path>] [--command <text>] [--window <id|ref|index>] [--focus <true|false>]"
        }
        "new-split" => {
            "new-split <left|right|up|down> [--workspace <id|ref|index>] [--surface <id|ref|index>] [--panel <id|ref|index>] [--window <id|ref|index>] [--command <text>] [--focus <true|false>]"
        }
        "focus-pane" => {
            "focus-pane --pane <id|ref|index> [--workspace <id|ref|index>] [--window <id|ref|index>]"
        }
        "focus-panel" => {
            "focus-panel --panel <id|ref|index> [--workspace <id|ref|index>] [--window <id|ref|index>]"
        }
        "select-workspace" => {
            "select-workspace --workspace <id|ref|index> [--window <id|ref|index>]"
        }
        "list-workspaces" => "list-workspaces [--window <id|ref|index>]",
        "list-panes" => "list-panes [--workspace <id|ref|index>] [--window <id|ref|index>]",
        "list-pane-surfaces" => {
            "list-pane-surfaces [--workspace <id|ref|index>] [--pane <id|ref|index>] [--window <id|ref|index>]"
        }
        "current-workspace" => "current-workspace [--window <id|ref|index>]",
        "send" => {
            "send [--workspace <id|ref|index>] [--surface <id|ref|index>] [--window <id|ref|index>] [--paste] <text>"
        }
        "send-key" => {
            "send-key [--workspace <id|ref|index>] [--surface <id|ref|index>] [--window <id|ref|index>] <key>"
        }
        "notify" => {
            "notify [--title <text>] [--subtitle <text>] [--body <text>] [--reply] [--desktop <true|false>] [--clear] [--workspace <id|ref|index>] [--surface <id|ref|index>] [--window <id|ref|index>]"
        }
        "dismiss-notification" => "dismiss-notification (--id <uuid> | --all-read)",
        "agent" => {
            "agent message <target> [--from <name>] <text|-> | agent message --reply-to <id> <text|-> | agent inbox [--surface <target>] [--state <state>] [--limit <n>] [--mark-read]"
        }
        "automation" => "automation <list|reload>",
        "vault" => "vault <list|resume <surface>>",
        "browser" => {
            "browser open <url> [--surface <id|ref|index>] [--workspace <id|ref|index>] [--window <id|ref|index>]"
        }
        "local-tmux" => "local-tmux <start|list|attach|close> [options]",
        "ssh" => "ssh <destination> [options]",
        "claude-teams" => "claude-teams [claude-args...]",
        "hooks" => {
            "hooks <setup|uninstall> [--settings-file <path>] | hooks <feed|message|session> --source claude"
        }
        "workspace-group" => {
            "workspace-group <list|create|ungroup|delete|rename|collapse|expand|pin|unpin|add|remove|set-anchor|new-workspace|focus> [options]"
        }
        "ping" => "ping",
        "version" => "version",
        _ => return None,
    })
}

pub(crate) fn top_level() -> String {
    let mut out = String::from(
        "cmux - control cmux via Unix socket

Usage:
  cmux [global-options] <command> [options]

Targets:
  Commands that accept a window, workspace, pane, or surface take a UUID, a short ref (window:1/workspace:2/pane:3/surface:4), or an index.
  Output defaults to refs; pass --id-format uuids or --id-format both to include UUIDs.

Socket Auth:
  --password takes precedence, then CMUX_SOCKET_PASSWORD, then the socket-control-password file.

Commands:
",
    );
    for (i, (group, cmds)) in GROUPS.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("  {group}:\n"));
        for cmd in *cmds {
            out.push_str(&format!("    {}\n", usage_line(cmd).unwrap_or(cmd)));
        }
    }
    out.push_str(
        "
Environment:
  CMUX_WORKSPACE_ID   Auto-set in cmux terminals. Used as default --workspace for
                      ALL commands (send, list-panes, new-split, notify, etc.).
  CMUX_SURFACE_ID     Auto-set in cmux terminals. Used as default --surface.
  CMUX_SOCKET_PATH    Override the Unix socket path. Without this, the CLI defaults
                      to ~/.local/state/cmux-linux/cmux.sock.
",
    );
    out
}

/// `cmux <cmd> --help`: the command name, a blank line, then the help text.
pub(crate) fn command(cmd: &str) -> Option<String> {
    let text = match cmd {
        "notify" => NOTIFY,
        "dismiss-notification" => DISMISS,
        "hooks" => HOOKS,
        "new-workspace" => NEW_WORKSPACE,
        "new-split" => NEW_SPLIT,
        "send" => SEND,
        "send-key" => SEND_KEY,
        "list-workspaces" => LIST_WORKSPACES,
        "current-workspace" => CURRENT_WORKSPACE,
        "list-panes" => LIST_PANES,
        "list-pane-surfaces" => LIST_PANE_SURFACES,
        "select-workspace" => SELECT_WORKSPACE,
        "focus-pane" => FOCUS_PANE,
        "focus-panel" => FOCUS_PANEL,
        "workspace-group" => WORKSPACE_GROUP,
        "agent" => AGENT,
        "automation" => AUTOMATION,
        "vault" => VAULT,
        "browser" => BROWSER,
        "local-tmux" => LOCAL_TMUX,
        "ssh" => SSH,
        "claude-teams" => CLAUDE_TEAMS,
        "ping" => PING,
        _ => return None,
    };
    Some(format!("cmux {cmd}\n\n{text}"))
}

const NOTIFY: &str = r#"Usage: cmux notify [flags]

Send a notification to a workspace/surface, or clear that resolved target with --clear.

Flags:
  --title <text>         Notification title (default: "Notification")
  --subtitle <text>      Notification subtitle
  --body <text>          Notification body
  --reply                Accepted for compatibility; inline replies are not supported
  --desktop <true|false> Post a desktop notification (default: true). false keeps the entry in the Notifications panel, sidebar badge and pane ring without a desktop notification
  --clear                Clear notifications for the resolved caller/target instead of posting
  --workspace <id|ref|index>   Target workspace, except explicit surface UUIDs resolve globally
  --surface <id|ref|index>     Target surface (refs/indexes use workspace/window context)
  --window <id|ref|index>      Window context for workspace/surface refs and indexes
  --json                 Print the response payload as JSON
  --id-format <mode>     refs, uuids, or both for human-readable ids

The response includes the created notification id. Use cmux dismiss-notification --id <uuid|notification:<uuid>> to remove it.
--desktop has no effect with --clear.

Example:
  cmux notify --title "Build done" --body "All tests passed"
  cmux notify --title "Error" --subtitle "test.swift" --body "Line 42: syntax error"
  cmux notify --surface <uuid> --title "Build done"
  cmux notify --title "Deploy finished" --desktop false
  cmux notify --clear
"#;

const HOOKS: &str = "Usage: cmux hooks <setup|uninstall> [--settings-file <path>]
       cmux hooks <feed|message|session> --source claude

  setup      Add cmux's hooks to Claude Code's settings
  uninstall  Remove cmux's hooks from Claude Code's settings
  feed       Run as Claude Code's PermissionRequest hook: show the prompt in the cmux Feed
  message    Run as Claude Code's UserPromptSubmit, PostToolUse and Stop hook: hand over
             the messages other agents left with `cmux agent message`; on Stop also
             report the turn's end for workspace auto-naming
  session    Run as Claude Code's SessionStart and SessionEnd hook: record which Claude
             session runs in this terminal, so it resumes after an app restart
";

const DISMISS: &str = "Usage: cmux dismiss-notification (--id <uuid> | --all-read)

Remove one notification, or remove every already-read notification.

Flags:
  --id <uuid|notification:<uuid>>  Notification id or notify handle to remove
  --all-read            Remove every already-read notification
  --json                Print JSON
  --id-format <mode>    refs, uuids, or both
";

const NEW_WORKSPACE: &str = r#"Usage: cmux new-workspace [--name <title>] [--cwd <path>] [--command <text>] [--window <id|ref|index>] [--focus <true|false>]

Create a new workspace in the caller's window.

Flags:
  --name <title>       Set a custom name for the new workspace
  --cwd <path>         Set the working directory for the new workspace
  --command <text>     Run this as the new terminal's initial command
  --window <id|ref|index> Target window (default: caller's window)
  --focus <true|false> Focus the new workspace (default: false)

Example:
  cmux new-workspace
  cmux new-workspace --name "Build Server"
  cmux new-workspace --cwd ~/projects/myapp
  cmux new-workspace --cwd . --command "npm test"
"#;

const NEW_SPLIT: &str = "Usage: cmux new-split <left|right|up|down> [flags]

Split the current pane in the given direction.

Flags:
  --workspace <id|ref>   Target workspace (default: $CMUX_WORKSPACE_ID)
  --surface <id|ref>     Surface to split from (default: $CMUX_SURFACE_ID)
  --panel <id|ref>       Alias for --surface
  --window <id|ref|index>
                          Window context for workspace/surface refs and indexes
  --command <text>     Run this as the new terminal's initial command
  --focus <true|false>   Focus the new split (default: false)

Example:
  cmux new-split right
  cmux new-split down --workspace workspace:1
";

const SEND: &str = r#"Usage: cmux send [flags] [--] <text>

Send text to a terminal surface as keystrokes. Escape sequences: \n and \r send Enter, \t sends Tab.

For large text (for example over 4 KB, or several lines going to an agent), use --paste. Keystrokes press Enter at every newline, which can submit each line on its own.

With --paste, the text goes through the paste path, unchanged: escape sequences such as \n are not interpreted, and control characters such as Esc and Ctrl-C are replaced with spaces. --paste must come before the text.

Flags:
  --workspace <id|ref|index>   Target workspace (default: $CMUX_WORKSPACE_ID)
  --surface <id|ref|index>     Target surface (default: $CMUX_SURFACE_ID)
  --window <id|ref|index>      Window context for workspace/surface refs and indexes
  --paste                      Paste the text instead of typing it

Example:
  cmux send "echo hello"
  cmux send --surface surface:2 "ls -la\n"
  cmux send --paste --surface surface:2 "$(cat notes.md)"
"#;

const SEND_KEY: &str = "Usage: cmux send-key [flags] [--] <key>

Send a key event to a terminal surface.

Flags:
  --workspace <id|ref|index>   Target workspace (default: $CMUX_WORKSPACE_ID)
  --surface <id|ref|index>     Target surface (default: $CMUX_SURFACE_ID)
  --window <id|ref|index>      Window context for workspace/surface refs and indexes

Example:
  cmux send-key enter
  cmux send-key --surface surface:2 ctrl+c
";

const LIST_WORKSPACES: &str = "Usage: cmux list-workspaces [--window <id|ref|index>]

List workspaces in a window.

Flags:
  --window <id|ref|index>   Target window (default: caller/current window)

Example:
  cmux list-workspaces
";

const CURRENT_WORKSPACE: &str = "Usage: cmux current-workspace [--window <id|ref|index>]

Print the selected workspace ID for a window.
";

const LIST_PANES: &str =
    "Usage: cmux list-panes [--workspace <id|ref|index>] [--window <id|ref|index>]

List panes in a workspace.

Flags:
  --workspace <id|ref|index>   Workspace context (default: $CMUX_WORKSPACE_ID)
  --window <id|ref|index>      Window context for workspace refs and indexes

Example:
  cmux list-panes
  cmux list-panes --workspace workspace:2
";

const LIST_PANE_SURFACES: &str = "Usage: cmux list-pane-surfaces [--workspace <id|ref|index>] [--pane <id|ref|index>] [--window <id|ref|index>]

List surfaces in a pane.

Flags:
  --workspace <id|ref|index>   Workspace context (default: $CMUX_WORKSPACE_ID)
  --pane <id|ref|index>        Restrict to a specific pane (default: focused pane)
  --window <id|ref|index>      Window context for workspace/pane refs and indexes

Example:
  cmux list-pane-surfaces
  cmux list-pane-surfaces --workspace workspace:2 --pane pane:1
";

const SELECT_WORKSPACE: &str =
    "Usage: cmux select-workspace --workspace <id|ref|index> [--window <id|ref|index>]

Select (switch to) the specified workspace.

Flags:
  --workspace <id|ref|index>   Workspace to select (required)
  --window <id|ref|index>      Window context for workspace refs and indexes

Example:
  cmux select-workspace --workspace workspace:2
  cmux select-workspace --workspace 0
";

const FOCUS_PANE: &str = "Usage: cmux focus-pane [--pane <id|ref|index> | <id|ref|index>] [flags]

Focus the specified pane.

Flags:
  --pane <id|ref|index>       Pane to focus (required unless passed positionally)
  --workspace <id|ref|index>  Workspace context (default: $CMUX_WORKSPACE_ID)
  --window <id|ref|index>     Window context for workspace/pane refs and indexes

Example:
  cmux focus-pane --pane pane:2
  cmux focus-pane pane:1
  cmux focus-pane --pane pane:1 --workspace workspace:2
";

const FOCUS_PANEL: &str = "Usage: cmux focus-panel --panel <id|ref|index> [--workspace <id|ref|index>] [--window <id|ref|index>]

Focus a specific panel (surface).

Flags:
  --panel <id|ref|index>       Panel/surface to focus (required)
  --workspace <id|ref|index>   Workspace context (default: $CMUX_WORKSPACE_ID)
  --window <id|ref|index>      Window context for workspace/panel refs and indexes

Example:
  cmux focus-panel --panel surface:2
  cmux focus-panel --panel surface:5 --workspace workspace:2
";

const PING: &str = "Usage: cmux ping

Check that cmux is running and reachable over its socket. Prints PONG.
";

const WORKSPACE_GROUP: &str = r#"Usage: cmux workspace-group <subcommand> [options]

Collapsible named sections of workspaces in the sidebar. The first workspace of a
group is its anchor: the group's header stands for it.

Subcommands:
  list
  create [--name <name>] [--cwd <path>] [--from <workspace>,<workspace>]
  ungroup <group>                       Dissolve the group; keep its workspaces.
  delete <group> [--close-workspaces]   Dissolve, or close every workspace in it.
  rename <group> --name <name>
  collapse <group> | expand <group>
  pin <group> | unpin <group>
  add --group <group> --workspace <id|ref|index>
  remove --workspace <id|ref|index>
  set-anchor --group <group> --workspace <id|ref|index>
  new-workspace <group> [--placement afterCurrent|top|end]
  focus <group>

A group is a UUID or a ref like workspace_group:1. Without --from, create makes a
group with a new anchor workspace.

Example:
  cmux workspace-group create --name api --from workspace:1,workspace:2
  cmux workspace-group new-workspace workspace_group:1
  cmux workspace-group list
"#;

const CLAUDE_TEAMS: &str = "Usage: cmux claude-teams [claude-args...]

Launch Claude Code with agent teams enabled. Run it in a cmux-linux terminal.

This command:
  - defaults Claude teammate mode to auto
  - sets a tmux-like environment so Claude auto mode uses cmux splits
  - sets CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1
  - prepends a private tmux shim to PATH
  - forwards all remaining arguments to claude

The tmux shim translates the tmux pane commands a team uses (split-window, send-keys,
kill-pane, list-panes, display-message, select-pane) into cmux splits in this workspace.

Examples:
  cmux claude-teams
  cmux claude-teams --continue
  cmux claude-teams --model sonnet
";

const AUTOMATION: &str = "Usage: cmux automation <list|reload>

Rules in ~/.config/cmux-linux/automations.json run actions when the app emits an event.

  list    Show the loaded rules, how often each fired, and why any rule was left out
  reload  Read automations.json again
";

const BROWSER: &str = "Usage: cmux browser open <url> [flags]

Open <url> (http, https or file) in a new browser tab in the pane of the target surface
(default: the caller's surface, else the focused one). Needs an app built with the
`browser` feature.

Flags:
  --surface <id|ref|index>     Surface whose pane gets the tab
  --workspace <id|ref|index>   Workspace (default: its focused surface)
  --window <id|ref|index>      Window
";

const VAULT: &str = "Usage: cmux vault list
       cmux vault resume <surface>

Agent sessions that resume when the app restarts: Claude Code sessions reported by
its hooks (`cmux hooks setup`) and agents from [[vault.agents]] in
~/.config/cmux-linux/config.toml, found by their running process.

  list    Show each session: surface, workspace, agent, session id, directory
  resume  Type the session's resume command into its surface again
";

const SSH: &str = "Usage: cmux ssh <destination> [flags]

Create a new workspace whose terminals (the first, and every split and tab) run ssh to
<destination>. They share one connection, so a password or host-key prompt shows only
in the first terminal. When ssh exits the terminal closes, as with a shell.

Flags:
  --name <title>          Workspace title (default: <destination>)
  --port <n>              SSH port
  --identity <path>       SSH identity file path
  --ssh-option <opt>      Extra SSH -o option (repeatable)
  --command <text>        Typed into the first remote terminal once it opens
  --focus <true|false>    Switch to the new workspace (default: true)
  --window <id|ref|index> Target window

Image files dropped on a remote terminal are uploaded to ~/.cache/cmux/paste on the
host and their remote paths typed. The sidebar shows the host, the remote git branch
and the ports the remote user listens on.

Example:
  cmux ssh dev@my-host
  cmux ssh dev@my-host --name gpu-box --port 2222 --identity ~/.ssh/id_ed25519
  cmux ssh dev@my-host --ssh-option UserKnownHostsFile=/dev/null
";

const LOCAL_TMUX: &str =
    "Usage: cmux local-tmux start <name> [--cwd <dir>] [--command <cmd>] [--detached]
       cmux local-tmux list
       cmux local-tmux attach <name>
       cmux local-tmux close <name>

Terminals whose processes survive an app restart: each session lives on your own
tmux server (~/.local/state/cmux-linux/local-tmux/tmux.sock), and this terminal
attaches to it. Quitting the app or closing the tab only detaches; a restored
terminal attaches again. Needs tmux (or CMUX_LOCAL_TMUX_BIN). Names: letters,
digits, _ and -, at most 128.

  start   Create the session (10,000 lines of history) and attach this terminal,
          unless --detached
  list    Show the sessions on the server
  attach  Attach this terminal to an existing session
  close   End the session and everything running in it
";

const AGENT: &str = r#"Usage: cmux agent message <target> [--from <name>] <text|->
       cmux agent message --reply-to <message-id> <text|->
       cmux agent inbox [--surface <target>] [--state queued|delivered|read] [--limit <n>] [--mark-read]

Leave a message for the agent in another workspace or surface. It is delivered
through that agent's hooks (install them with `cmux hooks setup`), never typed
into its terminal: a busy agent sees it at its next step, an idle one with its
next prompt.

The target is a workspace or surface id or ref, or a workspace title (exact
match first, then a unique prefix). A workspace means the surface running its
agent. --from defaults to the sending workspace's title. `-` reads the text
from standard input.

Example:
  cmux agent message workspace:4 --from reviewer "Review posted on #123."
  echo "long text" | cmux agent message surface:12 -
  cmux agent message --reply-to 3F2A... "Done."
  cmux agent inbox --state queued
"#;
