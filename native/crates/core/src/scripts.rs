//! The shell scripts burrow runs on the machines, word for word as server.py has them, and the names a
//! bot started here gets.

use crate::util::sq;
use std::sync::LazyLock;

/// a message between sessions shows up in the receiver's transcript as a user turn with
/// "origin":{"kind":"peer","from":"uds:…","verifiedPeerPid":…,"msg_id":"…","name":"<sender>",…};
/// grep the last 4 MB of transcripts written in the last minute (a session can write a few hundred kB
/// right after the message) and print "<receiver session id> <origin…>" per message.
/// The same grep finds a bot's traffic with a pair room: pair's agents talk with curl to <pair>/a/<token>/…, so a
/// post (/say) or a read (/messages) is a Bash tool call (its id), a wait that brings messages is pair's answer
/// ending in "last message #N … Next: reply if you have something to say"; and the answer to a join names the room
/// ("Room: X") and, further on, its token: which table it is (the one linked to that room)
macro_rules! pair_grep {
    () => {
        r#" -e '"id":"toolu_[A-Za-z0-9]+","name":"Bash","input":\{"command":"curl[^"]*/a/[A-Za-z0-9_-]+/(say|messages)' -e 'last message #[0-9]+\\nNext: reply if you have something to say \(POST /say\), then keep listening:\\n  curl -s .https?://[^/]+/a/[A-Za-z0-9_-]+/wait' -e 'Room: [^\\"]*\\nYou are: ' -e 'curl -s .https?://[^/]+/a/[A-Za-z0-9_-]+/messages.\\n  Wait for new messages' "#
    };
}
pub const PAIR_GREP: &str = pair_grep!();
pub const PEERS: &str = concat!(
    r#"find ~/.claude/projects -maxdepth 2 -name '*.jsonl' -mmin -1 2>/dev/null | while IFS= read -r f; do s=${f##*/}; tail -c 4194304 "$f" | grep -o -E -e '"origin":\{"kind":"peer"[^}]*' "#,
    pair_grep!(),
    r#"| sed "s|^|${s%.jsonl} |"; done"#
);

/// a session's whole transcript, for when it joined its pair room (longer ago than the last minutes): "<session> …"
pub fn pair_join(session: &str) -> String {
    format!(r#"for f in ~/.claude/projects/*/{session}.jsonl; do [ -f "$f" ] && grep -o -E{PAIR_GREP}"$f" | sed "s|^|{session} |"; done"#)
}

/// each background session's record (~/.claude/jobs/<id>/state.json): when it was last written, agent
/// view's one-line summary, its Remote Control session, its tempo, the conversation it goes on with and
/// the worktree it moved into; one grep, a `<path>:"<key>": "<value>"` a line
pub const JOBS: &str = r#"grep -o -E '"(updatedAt|detail|bridgeSessionId|tempo|resumeSessionId|worktreePath)": *"([^"\\]|\\.)*"' ~/.claude/jobs/*/state.json 2>/dev/null || true"#;

/// where Claude Code ran lately, for a new desk's form: its home, then the working directory of the
/// newest transcript of each of its projects, the latest first
pub const RECENT: &str = r#"echo "$HOME"; cd ~/.claude/projects 2>/dev/null || exit 0
for d in $(ls -t | head -100); do
  f=$(ls -t ./"$d"/*.jsonl 2>/dev/null | head -1); [ -n "$f" ] || continue
  c=$(grep -m1 -o '"cwd":"[^"]*"' "$f" | sed 's/^"cwd":"//; s/"$//')
  [ -n "$c" ] && [ -d "$c" ] && echo "$c"
done; true"#;

/// bytes of a transcript read, the newest
pub const TAIL: usize = 4_000_000;

/// a session's record and its transcript's size; the transcript's tail only if it is not `size` long any more
pub fn slice(id: &str, sid: &str, size: i64) -> String {
    format!(
        r#"cat ~/.claude/jobs/{id}/state.json 2>/dev/null; echo; echo '@@burrow@@'; f=$(ls ~/.claude/projects/*/{sid}.jsonl 2>/dev/null | head -1); if [ -n "$f" ]; then n=$(wc -c < "$f" | tr -d ' '); echo "$n"; [ "$n" != "{size}" ] && tail -c {TAIL} "$f"; fi; true"#,
        id = sq(id),
        sid = sq(if sid.is_empty() { "-" } else { sid }),
    )
}

/// a session in a git worktree sits at its repo's desk: for each new working directory, its worktree's
/// top and the repo's .git (resolved); a linked worktree is one whose top isn't the repo's
pub fn fold(dirs: &[String]) -> String {
    let ds: Vec<String> = dirs.iter().map(|d| sq(d)).collect();
    format!(
        r#"for d in {}; do t=$(cd "$d" 2>/dev/null && git rev-parse --show-toplevel 2>/dev/null); c=$(cd "$d" 2>/dev/null && cd "$(git rev-parse --git-common-dir 2>/dev/null)" 2>/dev/null && pwd -P); printf '%s\t%s\n' "$t" "$c"; done"#,
        ds.join(" ")
    )
}

/// a bot started here gets a name as pair gives its agents (rdaems/pair-prompting BOT_WORDS): one not in use
/// on that machine. A session named by Claude Code itself keeps its name
const BOT_WORDS: &[(&str, &str)] = &[
    ("bouncy brave breezy bubbly bold", "bob bea bert babs benny"),
    ("cheery clever cosmic cozy curious", "cleo carl cora chip cass"),
    ("dapper daring dizzy dreamy dandy", "dan dot dora dex dolly"),
    ("fancy fizzy frisky funky fuzzy", "fred flo finn fern felix"),
    ("giddy gentle groovy gutsy gleeful", "gus gina gil greta gary"),
    ("happy hasty humble hearty hip", "hank hattie hugo hal hazel"),
    ("jazzy jolly jumpy jovial jaunty", "jim june jojo jack jade"),
    ("lucky lively loyal lanky lofty", "lou lola leo lily lars"),
    ("merry mellow mighty misty mild", "max mona milo mabel moe"),
    ("nifty nimble noble nutty nosy", "ned nora nico nell nate"),
    ("perky plucky peppy polite posh", "pete pam pip polly perry"),
    ("rowdy rosy rusty radiant rapid", "rex rosie rudy ruth ray"),
    ("sunny snappy spunky sleepy swanky", "sam sue stan sid suzy"),
    ("tidy tiny trusty toasty twinkly", "tom tess ted tina toby"),
    ("witty wobbly wacky wise wily", "walt wanda wes winnie wally"),
    ("zany zesty zippy zen zealous", "zed zoe ziggy zara zack"),
];

pub static BOT_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| {
    BOT_WORDS
        .iter()
        .flat_map(|(adj, names)| adj.split(' ').flat_map(move |a| names.split(' ').map(move |n| format!("{a}-{n}"))))
        .collect()
});

#[cfg(test)]
mod tests {
    #[test]
    fn names_as_pairs() {
        assert_eq!(super::BOT_NAMES.len(), 16 * 25);
        assert_eq!(super::BOT_NAMES[0], "bouncy-bob");
        assert!(super::BOT_NAMES.contains(&"clever-cleo".to_string()));
    }

    #[test]
    fn the_fold_script() {
        let s = super::fold(&["/a b".into(), "/c".into()]);
        assert!(s.starts_with("for d in '/a b' '/c'; do") && s.contains(r"printf '%s\t%s\n'"));
    }
}
