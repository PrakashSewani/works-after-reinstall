use std::cell::Cell;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use anyhow::{Result, bail};
use ratatui::Frame;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph, Wrap};

use crate::catalog;
use crate::cli::Component;
use crate::commands;
use crate::console::Console;
use crate::discovery;
use crate::prompt;

const HOME_ITEMS: [&str; 4] = [
    "Start fresh - pick what to install",
    "Setup from a bundle",
    "Export this machine",
    "Quit",
];
const EXPORT_ROWS: usize = 6;
const SETUP_ROWS: usize = 7;
const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];
const MAX_LOG_LINES: usize = 500;
const LISTED_BUNDLES: usize = 5;
const PAGE: isize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Home,
    Starter,
    Export,
    Setup,
    Running,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StarterToggle {
    Windows,
    Environment,
    DryRun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StarterRow {
    Header(&'static str),
    App(usize),
    Toggle(StarterToggle),
    Action,
}

struct Running {
    title: String,
    lines: Receiver<String>,
    result: Receiver<std::result::Result<(), String>>,
    done: Option<std::result::Result<(), String>>,
    spinner: usize,
}

struct App {
    screen: Screen,
    focus: usize,
    bundles: Vec<PathBuf>,
    bundle_index: usize,
    export_output: String,
    export_enabled: [bool; 4],
    setup_enabled: [bool; 4],
    starter_rows: Vec<StarterRow>,
    starter_picked: Vec<bool>,
    starter_focus: usize,
    starter_scroll: usize,
    starter_windows: bool,
    starter_environment: bool,
    starter_dry_run: bool,
    dry_run: bool,
    notice: Option<String>,
    log: Vec<String>,
    log_scroll: usize,
    view_height: Cell<usize>,
    running: Option<Running>,
    should_quit: bool,
}

pub fn can_run() -> bool {
    prompt::is_interactive()
}

pub fn run() -> Result<()> {
    if !can_run() {
        bail!("the interactive interface needs a terminal - run a command like `starter` instead");
    }

    ratatui::run(|terminal| -> Result<()> {
        let _mouse = MouseCapture::enable()?;
        let mut app = App::new();
        while !app.should_quit {
            terminal.draw(|frame| app.draw(frame))?;
            app.tick(Duration::from_millis(100))?;
        }
        Ok(())
    })
}

/// Enables wheel events for as long as it lives, and puts the terminal back the
/// way it was even if the app panics.
struct MouseCapture;

impl MouseCapture {
    fn enable() -> Result<Self> {
        execute!(std::io::stdout(), EnableMouseCapture)?;
        Ok(MouseCapture)
    }
}

impl Drop for MouseCapture {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
    }
}

impl App {
    fn new() -> Self {
        let starter_rows = starter_rows();
        Self {
            screen: Screen::Home,
            focus: 0,
            bundles: discovery::find_bundles(),
            bundle_index: 0,
            export_output: discovery::DEFAULT_BUNDLE_DIR.to_string(),
            export_enabled: [true; 4],
            setup_enabled: [true; 4],
            starter_focus: first_selectable(&starter_rows),
            starter_rows,
            starter_picked: vec![false; catalog::APPS.len()],
            starter_scroll: 0,
            starter_windows: true,
            starter_environment: true,
            starter_dry_run: false,
            dry_run: false,
            notice: None,
            log: Vec::new(),
            log_scroll: 0,
            view_height: Cell::new(20),
            running: None,
            should_quit: false,
        }
    }

    fn tick(&mut self, timeout: Duration) -> Result<()> {
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key),
                Event::Mouse(mouse) => self.on_mouse(mouse),
                _ => {}
            }
        }
        self.drain();
        Ok(())
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        let up = match mouse.kind {
            MouseEventKind::ScrollUp => true,
            MouseEventKind::ScrollDown => false,
            _ => return,
        };
        match self.screen {
            // Wheel up walks back through the output, but up through a list.
            Screen::Running => self.scroll_log(if up { 1 } else { -1 }),
            Screen::Starter => self.starter_move(if up { -1 } else { 1 }),
            _ => {}
        }
    }

    fn drain(&mut self) {
        let mut received = Vec::new();
        let mut finished = None;

        if let Some(running) = self.running.as_ref() {
            while let Ok(line) = running.lines.try_recv() {
                received.push(line);
            }
            if let Ok(result) = running.result.try_recv() {
                finished = Some(result);
            }
        }

        let added = received.len();
        self.log.extend(received);
        if self.log.len() > MAX_LOG_LINES {
            self.log.drain(..self.log.len() - MAX_LOG_LINES);
        }
        if added > 0 && self.log_scroll > 0 {
            // Keep the line you are reading where it is while more output lands.
            self.log_scroll += added;
        }
        self.log_scroll = self.log_scroll.min(self.max_log_scroll());

        if let Some(result) = finished {
            if let Some(running) = self.running.as_mut() {
                running.done = Some(result);
            }
            self.refresh_bundles();
        } else if let Some(running) = self.running.as_mut() {
            running.spinner = running.spinner.wrapping_add(1);
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match self.screen {
            Screen::Home => self.on_home_key(key),
            Screen::Starter => self.on_starter_key(key),
            Screen::Export => self.on_export_key(key),
            Screen::Setup => self.on_setup_key(key),
            Screen::Running => self.on_running_key(key),
        }
    }

    fn on_home_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.focus = self.focus.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                self.focus = (self.focus + 1).min(HOME_ITEMS.len() - 1);
            }
            KeyCode::Enter => match self.focus {
                0 => self.open(Screen::Starter),
                1 => self.open(Screen::Setup),
                2 => self.open(Screen::Export),
                _ => self.should_quit = true,
            },
            _ => {}
        }
    }

    fn on_export_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.back_home(),
            KeyCode::Down | KeyCode::Tab => self.focus = (self.focus + 1) % EXPORT_ROWS,
            KeyCode::Up | KeyCode::BackTab => {
                self.focus = (self.focus + EXPORT_ROWS - 1) % EXPORT_ROWS;
            }
            KeyCode::Char(character)
                if self.focus == 0 && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.export_output.push(character);
            }
            KeyCode::Backspace if self.focus == 0 => {
                self.export_output.pop();
            }
            KeyCode::Char(' ') => toggle(&mut self.export_enabled, self.focus),
            KeyCode::Enter if self.focus == EXPORT_ROWS - 1 => self.start_export(),
            KeyCode::Enter => toggle(&mut self.export_enabled, self.focus),
            _ => {}
        }
    }

    fn on_setup_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.back_home(),
            KeyCode::Down | KeyCode::Tab => self.focus = (self.focus + 1) % SETUP_ROWS,
            KeyCode::Up | KeyCode::BackTab => {
                self.focus = (self.focus + SETUP_ROWS - 1) % SETUP_ROWS;
            }
            KeyCode::Left if self.focus == 0 => self.bundle_index = self.previous_bundle(),
            KeyCode::Right if self.focus == 0 => self.bundle_index = self.next_bundle(),
            KeyCode::Enter | KeyCode::Char(' ') if self.focus == SETUP_ROWS - 1 => {
                self.start_setup();
            }
            KeyCode::Enter | KeyCode::Char(' ') if self.focus == SETUP_ROWS - 2 => {
                self.dry_run = !self.dry_run;
            }
            KeyCode::Enter | KeyCode::Char(' ') => toggle(&mut self.setup_enabled, self.focus),
            _ => {}
        }
    }

    fn on_starter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.back_home(),
            KeyCode::Up | KeyCode::Char('k') => self.starter_move(-1),
            KeyCode::Down | KeyCode::Char('j') => self.starter_move(1),
            KeyCode::PageUp => self.starter_page(-PAGE),
            KeyCode::PageDown => self.starter_page(PAGE),
            KeyCode::Home => self.starter_jump(first_selectable(&self.starter_rows)),
            KeyCode::End => self.starter_jump(last_selectable(&self.starter_rows)),
            KeyCode::Char(' ') => self.starter_toggle_row(),
            KeyCode::Enter if self.starter_focus == self.starter_rows.len() - 1 => {
                self.start_starter();
            }
            KeyCode::Enter => self.starter_toggle_row(),
            _ => {}
        }
    }

    fn starter_move(&mut self, direction: isize) {
        let mut index = self.starter_focus as isize;
        loop {
            index += direction;
            if index < 0 || index as usize >= self.starter_rows.len() {
                break;
            }
            if matches!(self.starter_rows[index as usize], StarterRow::Header(_)) {
                continue;
            }
            self.starter_focus = index as usize;
            break;
        }
        self.starter_reveal();
    }

    fn starter_page(&mut self, delta: isize) {
        for _ in 0..delta.abs() {
            self.starter_move(delta.signum());
        }
    }

    fn starter_jump(&mut self, index: usize) {
        self.starter_focus = index;
        self.starter_reveal();
    }

    fn starter_reveal(&mut self) {
        let height = self.view_height.get().max(1);
        if self.starter_focus < self.starter_scroll {
            self.starter_scroll = self.starter_focus;
        } else if self.starter_focus >= self.starter_scroll + height {
            self.starter_scroll = self.starter_focus + 1 - height;
        }
    }

    fn starter_toggle_row(&mut self) {
        match self.starter_rows.get(self.starter_focus).copied() {
            Some(StarterRow::App(index)) => {
                self.starter_picked[index] = !self.starter_picked[index]
            }
            Some(StarterRow::Toggle(StarterToggle::Windows)) => {
                self.starter_windows = !self.starter_windows;
            }
            Some(StarterRow::Toggle(StarterToggle::Environment)) => {
                self.starter_environment = !self.starter_environment;
            }
            Some(StarterRow::Toggle(StarterToggle::DryRun)) => {
                self.starter_dry_run = !self.starter_dry_run;
            }
            _ => {}
        }
    }

    fn start_starter(&mut self) {
        let ids: Vec<String> = catalog::APPS
            .iter()
            .enumerate()
            .filter(|(index, _)| self.starter_picked[*index])
            .map(|(_, app)| app.id.to_string())
            .collect();

        let mut only = Vec::new();
        if !ids.is_empty() {
            only.push(Component::Apps);
        }
        if self.starter_windows {
            only.push(Component::Windows);
        }
        if self.starter_environment {
            only.push(Component::Environment);
        }
        if only.is_empty() {
            self.notice = Some("Pick at least one app, or turn on one of the extras.".to_string());
            return;
        }

        let manifest = catalog::starter_manifest(
            &ids,
            catalog::StarterChoices {
                windows_settings: self.starter_windows,
                environment: self.starter_environment,
            },
        );
        let root = PathBuf::from(".");
        let dry_run = self.starter_dry_run;

        self.run_job("Start fresh", move |console| {
            commands::apply_manifest(&manifest, &root, dry_run, &only, &console)
        });
    }

    fn on_running_key(&mut self, key: KeyEvent) {
        let finished = self
            .running
            .as_ref()
            .map(|running| running.done.is_some())
            .unwrap_or(true);

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_log(1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_log(-1),
            KeyCode::PageUp => self.scroll_log(PAGE),
            KeyCode::PageDown => self.scroll_log(-PAGE),
            KeyCode::Home => self.log_scroll = self.max_log_scroll(),
            KeyCode::End => self.log_scroll = 0,
            KeyCode::Enter | KeyCode::Esc if finished => self.back_home(),
            _ => {}
        }
    }

    /// Scrolling is measured from the bottom: 0 keeps the newest line in view.
    fn scroll_log(&mut self, delta: isize) {
        let max = self.max_log_scroll() as isize;
        self.log_scroll = (self.log_scroll as isize + delta).clamp(0, max) as usize;
    }

    fn max_log_scroll(&self) -> usize {
        let room = self.view_height.get().saturating_sub(2);
        self.log.len().saturating_sub(room)
    }

    fn open(&mut self, screen: Screen) {
        self.refresh_bundles();
        self.screen = screen;
        self.focus = 0;
        self.notice = None;
        self.log_scroll = 0;
        if screen == Screen::Starter {
            self.starter_focus = first_selectable(&self.starter_rows);
            self.starter_scroll = 0;
        }
    }

    fn back_home(&mut self) {
        self.running = None;
        self.open(Screen::Home);
    }

    fn refresh_bundles(&mut self) {
        self.bundles = discovery::find_bundles();
        if self.bundle_index >= self.bundles.len() {
            self.bundle_index = 0;
        }
    }

    fn next_bundle(&self) -> usize {
        if self.bundles.is_empty() {
            0
        } else {
            (self.bundle_index + 1) % self.bundles.len()
        }
    }

    fn previous_bundle(&self) -> usize {
        if self.bundles.is_empty() {
            0
        } else {
            (self.bundle_index + self.bundles.len() - 1) % self.bundles.len()
        }
    }

    fn enabled_components(&self, selection: &[bool; 4]) -> Vec<Component> {
        Component::ALL
            .iter()
            .enumerate()
            .filter(|(index, _)| selection[*index])
            .map(|(_, component)| *component)
            .collect()
    }

    fn disabled_components(&self, selection: &[bool; 4]) -> Vec<Component> {
        Component::ALL
            .iter()
            .enumerate()
            .filter(|(index, _)| !selection[*index])
            .map(|(_, component)| *component)
            .collect()
    }

    fn start_export(&mut self) {
        if !self.export_enabled.iter().any(|enabled| *enabled) {
            self.notice = Some("Pick at least one thing to capture.".to_string());
            return;
        }
        let output = PathBuf::from(self.export_output.trim());
        if output.as_os_str().is_empty() {
            self.notice = Some("Give the bundle a folder name first.".to_string());
            return;
        }

        let skip = self.disabled_components(&self.export_enabled);
        self.run_job("Export", move |console| {
            commands::export(&output, &[], &skip, &console)
        });
    }

    fn start_setup(&mut self) {
        let Some(bundle) = self.bundles.get(self.bundle_index).cloned() else {
            self.notice = Some("No bundle found - export this machine first.".to_string());
            return;
        };

        let only = self.enabled_components(&self.setup_enabled);
        if only.is_empty() {
            self.notice = Some("Pick at least one thing to restore.".to_string());
            return;
        }

        let dry_run = self.dry_run;
        self.run_job("Setup", move |console| {
            commands::setup(&bundle, dry_run, &only, &console)
        });
    }

    fn run_job<F>(&mut self, title: &str, job: F)
    where
        F: FnOnce(Console) -> Result<()> + Send + 'static,
    {
        let (line_sender, lines) = channel();
        let (result_sender, result) = channel();

        self.log.clear();
        self.notice = None;
        self.screen = Screen::Running;
        self.running = Some(Running {
            title: title.to_string(),
            lines,
            result,
            done: None,
            spinner: 0,
        });

        let console = Console::channel(line_sender);
        std::thread::spawn(move || {
            let outcome = job(console).map_err(|error| format!("{error:#}"));
            let _ = result_sender.send(outcome);
        });
    }

    fn draw(&self, frame: &mut Frame) {
        match self.screen {
            Screen::Home => self.draw_home(frame),
            Screen::Starter => self.draw_starter(frame),
            Screen::Export => self.draw_export(frame),
            Screen::Setup => self.draw_setup(frame),
            Screen::Running => self.draw_running(frame),
        }
    }

    fn draw_home(&self, frame: &mut Frame) {
        let [body, footer] = split(frame.area());
        let mut lines = vec![
            Line::from("Your Windows setup, rebuilt after every reinstall."),
            Line::from(""),
        ];

        for (index, item) in HOME_ITEMS.iter().enumerate() {
            lines.push(row_line(index == self.focus, *item));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Bundles found nearby:"));
        if self.bundles.is_empty() {
            lines.push(warning_line(
                "  none yet - export this machine to create one",
            ));
        } else {
            for path in self.bundles.iter().take(LISTED_BUNDLES) {
                lines.push(Line::from(format!("  {}", path.display())));
            }
            if self.bundles.len() > LISTED_BUNDLES {
                lines.push(Line::from(format!(
                    "  ... and {} more",
                    self.bundles.len() - LISTED_BUNDLES
                )));
            }
        }

        if let Some(notice) = &self.notice {
            lines.push(Line::from(""));
            lines.push(warning_line(format!("  {notice}")));
        }

        panel(frame, body, " Works After Reinstall ", lines);
        footer_line(frame, footer, "Up/Down move   Enter select   q quit");
    }

    fn draw_export(&self, frame: &mut Frame) {
        let [body, footer] = split(frame.area());
        let mut lines = vec![
            row_line(self.focus == 0, "Bundle folder"),
            Line::from(format!("     {}", self.export_output)),
            Line::from(""),
            Line::from("   What to capture"),
        ];

        for (index, component) in Component::ALL.iter().enumerate() {
            lines.push(checkbox_line(
                self.focus == index + 1,
                self.export_enabled[index],
                component.title(),
            ));
        }

        lines.push(Line::from(""));
        lines.push(action_line(self.focus == EXPORT_ROWS - 1, "Start export"));

        if let Some(notice) = &self.notice {
            lines.push(Line::from(""));
            lines.push(warning_line(format!("  {notice}")));
        }

        panel(frame, body, " Export ", lines);
        footer_line(
            frame,
            footer,
            "Tab next   Space toggle   Enter start   Esc back",
        );
    }

    fn draw_setup(&self, frame: &mut Frame) {
        let [body, footer] = split(frame.area());
        let mut lines = vec![row_line(self.focus == 0, "Bundle")];

        match self.bundles.get(self.bundle_index) {
            Some(path) => {
                let hint = if self.bundles.len() > 1 {
                    format!(
                        "     < {} >   ({} of {})",
                        path.display(),
                        self.bundle_index + 1,
                        self.bundles.len()
                    )
                } else {
                    format!("     {}", path.display())
                };
                lines.push(Line::from(hint));
            }
            None => lines.push(warning_line(
                "     no bundle found - go back and export this machine first",
            )),
        }

        lines.push(Line::from(""));
        lines.push(Line::from("   What to restore"));
        for (index, component) in Component::ALL.iter().enumerate() {
            lines.push(checkbox_line(
                self.focus == index + 1,
                self.setup_enabled[index],
                component.title(),
            ));
        }

        lines.push(Line::from(""));
        lines.push(checkbox_line(
            self.focus == SETUP_ROWS - 2,
            self.dry_run,
            "Dry run (change nothing)",
        ));
        lines.push(action_line(self.focus == SETUP_ROWS - 1, "Start setup"));

        if let Some(notice) = &self.notice {
            lines.push(Line::from(""));
            lines.push(warning_line(format!("  {notice}")));
        }

        panel(frame, body, " Setup ", lines);
        footer_line(
            frame,
            footer,
            "Tab next   Space toggle   Left/Right bundle   Enter start   Esc back",
        );
    }

    fn draw_running(&self, frame: &mut Frame) {
        let [body, footer] = split(frame.area());
        let Some(running) = self.running.as_ref() else {
            return;
        };

        let status = match &running.done {
            Some(Ok(())) => Line::from(Span::styled(
                "Done.",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Some(Err(message)) => Line::from(Span::styled(
                format!("Failed: {message}"),
                Style::default().fg(Color::Red),
            )),
            None => Line::from(format!(
                "Working {}",
                SPINNER[running.spinner % SPINNER.len()]
            )),
        };

        let view = body.height.saturating_sub(2) as usize;
        self.view_height.set(view);
        let room = view.saturating_sub(2);
        let total = self.log.len();
        let end = total.saturating_sub(self.log_scroll.min(total));
        let start = end.saturating_sub(room);

        let mut lines: Vec<Line> = Vec::with_capacity(room + 2);
        for _ in 0..room.saturating_sub(end - start) {
            lines.push(Line::from(""));
        }
        for line in &self.log[start..end] {
            lines.push(Line::from(line.clone()));
        }
        lines.push(Line::from(""));
        lines.push(status);

        let title = if self.log_scroll > 0 {
            format!(" {} - scrolled back {} ", running.title, self.log_scroll)
        } else {
            format!(" {} ", running.title)
        };
        panel(frame, body, &title, lines);

        let hint = if running.done.is_some() {
            "Up/Down scroll   End newest   Enter back   Ctrl+C quit"
        } else {
            "Up/Down scroll   working, please wait   Ctrl+C quit"
        };
        footer_line(frame, footer, hint);
    }

    fn draw_starter(&self, frame: &mut Frame) {
        let [body, footer] = split(frame.area());
        let view = body.height.saturating_sub(2) as usize;
        self.view_height.set(view);

        let start = self.starter_scroll.min(self.starter_rows.len());
        let end = (start + view).min(self.starter_rows.len());
        let lines: Vec<Line> = (start..end).map(|index| self.starter_line(index)).collect();

        panel(frame, body, &self.starter_title(end), lines);
        footer_line(
            frame,
            footer,
            "Up/Down move   Space pick   PageUp/Down jump   Enter go   Esc back",
        );
    }

    fn starter_line(&self, index: usize) -> Line<'static> {
        let focused = index == self.starter_focus;
        match self.starter_rows[index] {
            StarterRow::Header(title) => Line::from(Span::styled(
                format!("   {title}"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            StarterRow::App(index) => {
                let entry = &catalog::APPS[index];
                let label = if entry.note.is_empty() {
                    entry.name.to_string()
                } else {
                    format!("{}  -  {}", entry.name, entry.note)
                };
                checkbox_line(focused, self.starter_picked[index], &label)
            }
            StarterRow::Toggle(StarterToggle::Windows) => checkbox_line(
                focused,
                self.starter_windows,
                "Windows: dark mode, file extensions, developer mode",
            ),
            StarterRow::Toggle(StarterToggle::Environment) => checkbox_line(
                focused,
                self.starter_environment,
                "Environment: EDITOR, .NET telemetry opt-out",
            ),
            StarterRow::Toggle(StarterToggle::DryRun) => checkbox_line(
                focused,
                self.starter_dry_run,
                "Dry run - show the plan, change nothing",
            ),
            StarterRow::Action => action_line(focused, "Set this machine up"),
        }
    }

    fn starter_title(&self, end: usize) -> String {
        let picked = self.starter_picked.iter().filter(|picked| **picked).count();
        let mut title = format!(" Start fresh - {picked} apps picked");
        if self.starter_scroll > 0 {
            title.push_str(" - more above");
        }
        if end < self.starter_rows.len() {
            title.push_str(" - more below");
        }
        title.push(' ');
        title
    }
}

fn split(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Min(5), Constraint::Length(1)]).areas(area)
}

fn starter_rows() -> Vec<StarterRow> {
    let mut rows = Vec::new();
    let mut category = "";
    for (index, app) in catalog::APPS.iter().enumerate() {
        if app.category != category {
            category = app.category;
            rows.push(StarterRow::Header(category));
        }
        rows.push(StarterRow::App(index));
    }
    rows.push(StarterRow::Header("Also set up"));
    rows.push(StarterRow::Toggle(StarterToggle::Windows));
    rows.push(StarterRow::Toggle(StarterToggle::Environment));
    rows.push(StarterRow::Toggle(StarterToggle::DryRun));
    rows.push(StarterRow::Action);
    rows
}

fn first_selectable(rows: &[StarterRow]) -> usize {
    rows.iter()
        .position(|row| !matches!(row, StarterRow::Header(_)))
        .unwrap_or(0)
}

fn last_selectable(rows: &[StarterRow]) -> usize {
    rows.iter()
        .rposition(|row| !matches!(row, StarterRow::Header(_)))
        .unwrap_or(0)
}

fn component_row(row: usize) -> Option<usize> {
    let index = row.checked_sub(1)?;
    (index < Component::ALL.len()).then_some(index)
}

fn toggle(selection: &mut [bool; 4], row: usize) {
    if let Some(index) = component_row(row) {
        selection[index] = !selection[index];
    }
}

fn row_line(selected: bool, label: impl Into<String>) -> Line<'static> {
    let label = label.into();
    if selected {
        Line::from(Span::styled(
            format!(" > {label}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(format!("   {label}"))
    }
}

fn checkbox_line(selected: bool, checked: bool, label: &str) -> Line<'static> {
    let marker = if checked { "[x]" } else { "[ ]" };
    row_line(selected, format!("{marker} {label}"))
}

fn action_line(selected: bool, label: &str) -> Line<'static> {
    if selected {
        row_line(true, label)
    } else {
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                label.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ])
    }
}

fn warning_line(text: impl Into<String>) -> Line<'static> {
    Line::from(Span::styled(
        text.into(),
        Style::default().fg(Color::Yellow),
    ))
}

fn panel(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line<'_>>) {
    let block = Block::bordered()
        .title(title.to_string())
        .padding(Padding::horizontal(1))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn footer_line(frame: &mut Frame, area: Rect, text: &str) {
    let line = Span::styled(format!(" {text}"), Style::default().fg(Color::DarkGray));
    frame.render_widget(Paragraph::new(Line::from(line)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn app() -> App {
        let mut app = App::new();
        app.bundles = Vec::new();
        app
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(app: &mut App, text: &str) {
        for character in text.chars() {
            press(app, KeyCode::Char(character));
        }
    }

    fn render(app: &App) -> String {
        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();

        let buffer = terminal.backend().buffer();
        let area = buffer.area;
        let mut screen = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                screen.push_str(buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "));
            }
            screen.push('\n');
        }
        screen
    }

    #[test]
    fn home_lists_the_things_you_can_do() {
        let screen = render(&app());
        assert!(screen.contains("Works After Reinstall"));
        assert!(screen.contains("Start fresh"));
        assert!(screen.contains("Export this machine"));
        assert!(screen.contains("Setup from a bundle"));
        assert!(screen.contains("Quit"));
        assert!(screen.contains("none yet"));
    }

    #[test]
    fn home_lists_bundles_it_found() {
        let mut app = app();
        app.bundles = vec![PathBuf::from(r"C:\bundles\one")];

        assert!(render(&app).contains(r"C:\bundles\one"));
    }

    #[test]
    fn home_opens_every_screen_and_comes_back() {
        let mut app = app();

        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Starter);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Home);

        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Setup);
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Export);
    }

    #[test]
    fn quit_is_reachable_from_home() {
        let mut app = app();
        for _ in 0..HOME_ITEMS.len() - 1 {
            press(&mut app, KeyCode::Down);
        }
        press(&mut app, KeyCode::Enter);
        assert!(app.should_quit);
    }

    #[test]
    fn export_screen_edits_the_bundle_folder() {
        let mut app = app();
        app.screen = Screen::Export;
        app.export_output.clear();

        type_text(&mut app, "my-bundle");
        assert_eq!(app.export_output, "my-bundle");

        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.export_output, "my-bundl");
    }

    #[test]
    fn export_screen_toggles_components() {
        let mut app = app();
        app.screen = Screen::Export;
        app.focus = 1;

        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.export_enabled, [false, true, true, true]);

        press(&mut app, KeyCode::Enter);
        assert_eq!(app.export_enabled, [true, true, true, true]);
    }

    #[test]
    fn export_screen_refuses_to_start_with_nothing_selected() {
        let mut app = app();
        app.screen = Screen::Export;
        app.focus = EXPORT_ROWS - 1;
        app.export_enabled = [false; 4];

        press(&mut app, KeyCode::Enter);

        assert_eq!(app.screen, Screen::Export);
        assert!(app.notice.is_some());
    }

    #[test]
    fn setup_screen_shows_the_bundle_and_can_toggle_dry_run() {
        let mut app = app();
        app.bundles = vec![PathBuf::from(r"C:\bundles\one")];
        app.screen = Screen::Setup;

        let screen = render(&app);
        assert!(screen.contains(r"C:\bundles\one"));
        assert!(screen.contains("Dry run"));

        app.focus = SETUP_ROWS - 2;
        press(&mut app, KeyCode::Char(' '));
        assert!(app.dry_run);
    }

    #[test]
    fn setup_screen_cycles_between_bundles() {
        let mut app = app();
        app.bundles = vec![
            PathBuf::from(r"C:\bundles\one"),
            PathBuf::from(r"C:\bundles\two"),
        ];
        app.screen = Screen::Setup;
        app.focus = 0;

        press(&mut app, KeyCode::Right);
        assert_eq!(app.bundle_index, 1);

        press(&mut app, KeyCode::Right);
        assert_eq!(app.bundle_index, 0);

        press(&mut app, KeyCode::Left);
        assert_eq!(app.bundle_index, 1);
    }

    #[test]
    fn setup_screen_refuses_to_start_without_a_bundle() {
        let mut app = app();
        app.screen = Screen::Setup;
        app.focus = SETUP_ROWS - 1;

        press(&mut app, KeyCode::Enter);

        assert_eq!(app.screen, Screen::Setup);
        assert!(app.notice.is_some());
    }

    #[test]
    fn running_screen_shows_progress_lines() {
        let (line_sender, lines) = channel();
        let (_result_sender, result) = channel();
        let mut app = app();
        app.screen = Screen::Running;
        app.running = Some(Running {
            title: "Export".to_string(),
            lines,
            result,
            done: None,
            spinner: 0,
        });

        line_sender.send("[apps]".to_string()).unwrap();
        line_sender.send("  36 packages".to_string()).unwrap();
        app.drain();

        let screen = render(&app);
        assert!(screen.contains("Export"));
        assert!(screen.contains("[apps]"));
        assert!(screen.contains("36 packages"));
        assert!(screen.contains("Working"));
    }

    #[test]
    fn running_screen_reports_failures() {
        let (_line_sender, lines) = channel();
        let (result_sender, result) = channel();
        let mut app = app();
        app.screen = Screen::Running;
        app.running = Some(Running {
            title: "Setup".to_string(),
            lines,
            result,
            done: None,
            spinner: 0,
        });

        result_sender
            .send(Err("winget is not available".to_string()))
            .unwrap();
        app.drain();

        assert!(render(&app).contains("Failed: winget is not available"));
    }

    #[test]
    fn running_screen_ignores_keys_until_the_job_finishes() {
        let (_line_sender, lines) = channel();
        let (_result_sender, result) = channel();
        let mut app = app();
        app.screen = Screen::Running;
        app.running = Some(Running {
            title: "Export".to_string(),
            lines,
            result,
            done: None,
            spinner: 0,
        });

        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Running);
    }

    fn running_app(count: usize) -> (App, std::sync::mpsc::Sender<String>) {
        let (sender, lines) = channel();
        let (_result_sender, result) = channel();
        let mut app = app();
        app.screen = Screen::Running;
        for index in 0..count {
            sender.send(format!("line {index}")).unwrap();
        }
        app.running = Some(Running {
            title: "Export".to_string(),
            lines,
            result,
            done: None,
            spinner: 0,
        });
        app.drain();
        (app, sender)
    }

    fn wheel(app: &mut App, kind: MouseEventKind) {
        app.on_mouse(MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
    }

    #[test]
    fn starter_screen_lists_categories_and_apps() {
        let mut app = app();
        app.screen = Screen::Starter;

        let screen = render(&app);

        assert!(screen.contains("Start fresh"));
        assert!(screen.contains("Browsers"));
        assert!(screen.contains("Chrome"));
        assert!(screen.contains("Git"));

        press(&mut app, KeyCode::End);
        let bottom = render(&app);
        assert!(bottom.contains("Set this machine up"));
        assert!(bottom.contains("Dry run"));
        assert!(bottom.contains("more above"));
    }

    #[test]
    fn starter_screen_picks_apps_and_reports_the_count() {
        let mut app = app();
        app.screen = Screen::Starter;

        press(&mut app, KeyCode::Char(' '));
        assert!(app.starter_picked[0]);
        assert!(render(&app).contains("1 apps picked"));

        press(&mut app, KeyCode::Char(' '));
        assert!(!app.starter_picked[0]);
        assert!(render(&app).contains("0 apps picked"));
    }

    #[test]
    fn starter_screen_steps_over_category_headers() {
        let mut app = app();
        app.screen = Screen::Starter;
        render(&app);

        assert_eq!(app.starter_focus, first_selectable(&app.starter_rows));

        press(&mut app, KeyCode::Up);
        assert_eq!(
            app.starter_focus,
            first_selectable(&app.starter_rows),
            "stops at the first app"
        );

        for _ in 0..5 {
            press(&mut app, KeyCode::Down);
        }
        assert!(matches!(
            app.starter_rows[app.starter_focus],
            StarterRow::App(_)
        ));
        assert!(matches!(
            app.starter_rows[app.starter_focus - 1],
            StarterRow::Header(_)
        ));
    }

    #[test]
    fn starter_screen_scrolls_to_keep_the_focus_visible() {
        let mut app = app();
        app.screen = Screen::Starter;
        render(&app);
        let view = app.view_height.get();

        for _ in 0..40 {
            press(&mut app, KeyCode::Down);
        }

        assert!(app.starter_scroll > 0, "the list should have scrolled");
        assert!(app.starter_focus >= app.starter_scroll);
        assert!(app.starter_focus < app.starter_scroll + view);
        assert!(render(&app).contains("more above"));
    }

    #[test]
    fn starter_screen_refuses_to_start_when_nothing_is_picked() {
        let mut app = app();
        app.screen = Screen::Starter;
        app.starter_windows = false;
        app.starter_environment = false;
        app.starter_focus = app.starter_rows.len() - 1;

        press(&mut app, KeyCode::Enter);

        assert_eq!(app.screen, Screen::Starter);
        assert!(app.notice.is_some());
    }

    #[test]
    fn starter_screen_runs_a_dry_run_to_the_end() {
        let mut app = app();
        app.screen = Screen::Starter;
        app.starter_dry_run = true;
        app.starter_picked[0] = true;
        app.starter_focus = app.starter_rows.len() - 1;

        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Running);

        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while app
            .running
            .as_ref()
            .is_some_and(|running| running.done.is_none())
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(25));
            app.drain();
        }

        assert!(
            app.running
                .as_ref()
                .is_some_and(|running| running.done.is_some()),
            "the starter run never reported back"
        );
        assert!(render(&app).contains("Dry run"));
    }

    #[test]
    fn running_screen_scrolls_back_through_the_log() {
        let (mut app, sender) = running_app(60);

        assert!(render(&app).contains("line 59"));

        press(&mut app, KeyCode::Home);
        assert!(app.log_scroll > 0);

        let oldest = render(&app);
        assert!(oldest.contains("line 0"));
        assert!(oldest.contains("scrolled back"));

        let before = app.log_scroll;
        sender.send("line 60".to_string()).unwrap();
        app.drain();
        assert_eq!(
            app.log_scroll,
            before + 1,
            "the view stays where you left it"
        );
        assert!(render(&app).contains("line 0"));

        press(&mut app, KeyCode::End);
        assert_eq!(app.log_scroll, 0);
        assert!(render(&app).contains("line 60"));
    }

    #[test]
    fn the_mouse_wheel_scrolls_the_log() {
        let (mut app, _sender) = running_app(60);

        wheel(&mut app, MouseEventKind::ScrollUp);
        assert!(app.log_scroll > 0);

        wheel(&mut app, MouseEventKind::ScrollDown);
        assert_eq!(app.log_scroll, 0);
    }

    #[test]
    fn the_mouse_wheel_moves_the_starter_list() {
        let mut app = app();
        app.screen = Screen::Starter;
        let first = app.starter_focus;

        wheel(&mut app, MouseEventKind::ScrollDown);
        assert!(app.starter_focus > first);

        wheel(&mut app, MouseEventKind::ScrollUp);
        assert_eq!(app.starter_focus, first);
    }

    #[test]
    fn each_screen_shows_its_own_key_hints() {
        let mut app = app();

        assert!(render(&app).contains("Enter select"));

        app.screen = Screen::Starter;
        assert!(render(&app).contains("Space pick"));

        app.screen = Screen::Export;
        assert!(render(&app).contains("Space toggle"));

        app.screen = Screen::Setup;
        assert!(render(&app).contains("Left/Right bundle"));

        let (running, _sender) = running_app(5);
        assert!(render(&running).contains("Up/Down scroll"));
    }
}
