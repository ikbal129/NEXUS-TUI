use std::collections::VecDeque;

use ratatui::{DefaultTerminal, Frame};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::{Block, Paragraph, BorderType, Gauge, Sparkline, LineGauge};
use ratatui::style::{Color, Style, Stylize};
use ratatui::layout::{Constraint, Layout, Rect, Alignment};
use ratatui::text::{Line, Span};
use sysinfo::{MINIMUM_CPU_UPDATE_INTERVAL, System, Disks};

use crate::sysmot::{level_color, percent, push_history};

const MUTED: Color = Color::DarkGray;
const ACCENT: Color = Color::Cyan;
const TITLE: &str = "N.E.X.U.S";
const HISTORY_LEN: usize = 120;

enum Menu {
    MainMenu,
    SystemMonitor,
    Pomodoro,
    TodoApp,
    NotesApp
}

mod sysmot;
struct App {
    running: bool,
    system: System,
    host_name: String,
    cpu_history: Vec<u64>,
    os_name: String,
    is_input: bool,
    input_buffer: String,
    mode: Menu,
    disks: Disks,
    cpu_history_sys: VecDeque<f64>
}

fn main() -> std::io::Result<()>{
    ratatui::run(|terminal: &mut DefaultTerminal| App::new().run(terminal))?;
    Ok(())
}

impl App {
    fn new() -> Self {
        let mut system = System::new_all().unwrap();
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_usage();
        let cpu_history = vec![0; 65];

        let mut app =  Self {
            running: true,
            system,
            host_name: System::host_name().unwrap_or_else(|err| "unknown".into()),
            cpu_history,
            os_name: System::name().unwrap_or_else(|err| "unknown os".into()),
            is_input: false,
            input_buffer: "".to_string(),
            mode: Menu::MainMenu,
            disks: Disks::new_with_refreshed_list().unwrap(),
            cpu_history_sys: VecDeque::with_capacity(HISTORY_LEN)
        };
        app.on_tick();
        app
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()>{
        while self.running {
            terminal.draw(|frame: &mut Frame| {
                match self.mode {
                    Menu::MainMenu => self.render(frame),
                    Menu::SystemMonitor => self.render_sysmot(frame),
                    _ => {}
                }
            }
        )?;

            if event::poll(std::time::Duration::from_millis(500))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key);
                }
            }

            self.on_tick();
        }

        Ok(())
    } 

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if self.is_input {
            match key.code {
                KeyCode::Esc => {
                    self.is_input = false;
                    self.input_buffer.clear();
                }
                KeyCode::Enter => {
                    self.is_input = false;
                    self.handle_input();
                    self.input_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            }

            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Char(':') => self.is_input = true,
            _ => {}
        }
    }

    fn on_tick(&mut self) {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh(true);
        push_history(&mut self.cpu_history_sys, self.system.global_cpu_usage() as f64);
        let cpu_val = self.system.global_cpu_usage().round() as u64;
        self.cpu_history.push(cpu_val);

        if self.cpu_history.len() > 30 {
            self.cpu_history.remove(0);
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let [header, top, bottom, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Length(3)
        ]).areas(frame.area());

        let [cpu, memory] = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50)
        ]).areas(top);

        let [todo, notes] = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50)
        ]).areas(bottom);

        self.header_panel(frame, header, "System Terminal");
        self.cpu_panel(frame, cpu);
        frame.render_widget(panel("Pomodoro"), memory);
        frame.render_widget(panel("To do List"), todo);
        frame.render_widget(panel("Notes"), notes);
        self.footer_panel(frame, footer);
    }

    fn header_panel(&mut self, frame: &mut Frame, area: Rect, title: &str) {
        let sep = Span::styled(" | ", MUTED);
        let uptime = System::uptime().unwrap();
        let line = Line::from(vec![
            "N.E.X.U.S".fg(ACCENT).bold(),
            sep.clone(),
            Span::raw(format!("{title}")),
            sep.clone(),
            Span::styled(format!("{}", self.host_name), Style::new().fg(MUTED).bold()),
            sep,
            Span::raw(format!("up {}h {}m", uptime / 3600, uptime % 3600 / 60))
        ]);
        let block = Block::bordered().border_style(MUTED).border_type(BorderType::Rounded);

        frame.render_widget(Paragraph::new(line).block(block).alignment(Alignment::Center), area);
    }

    fn footer_panel(&mut self, frame: &mut Frame, area: Rect) {
        let key = |k: &'static str| {
            Span::styled(format!(" {k} "), Style::new().fg(Color::Black).bg(ACCENT))
        };
        if self.is_input {
            let text = format!("{}|", self.input_buffer);
            let input_box = Paragraph::new(format!("{}", text))
            .block(Block::bordered().border_type(BorderType::Rounded).title(" Command "));
            
            frame.render_widget(input_box, area);
        } else {
            let exit = Line::from(vec![
                key("Esc"),
                Span::styled(" / ", MUTED),
                key("q"),
                Span::styled(" quit", MUTED)
            ]);
            frame.render_widget(Paragraph::new(exit), area);
        }
                
    }

    fn cpu_panel(&mut self, frame: &mut Frame, area: Rect) {
        let bar_background: Color = Color::Rgb(50, 50, 50);
        let cpu_block = panel("CPU");
        frame.render_widget(cpu_block.clone(), area);
        let [cpu, memory, history, os] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(1)
        ]).areas(cpu_block.inner(area));

        //cpu
        let cpu_usage = self.system.global_cpu_usage();
        let cpu_percent = cpu_usage.round()as f64;
        
        let cpu_gauge = Gauge::default()
                                .block(Block::bordered().title(" CPU Usage "))
                                .gauge_style(Style::default().fg(level_color(cpu_percent)).bg(bar_background))
                                .label(format!("{:.1}%", cpu_usage))
                                .percent(cpu_percent as u16);

        //ram
        let ram_usage = self.system.used_memory();
        let ram_total = self.system.total_memory();

        let ram_percent = if ram_total > 0 {
            ((ram_usage * 100) / ram_total) as u16
        } else {
            0
        };

        let ram_gauge = Gauge::default()
                                .block(Block::bordered().title(" RAM Usage "))
                                .gauge_style(Style::default().fg(level_color(percent(ram_usage, ram_total))).bg(bar_background))
                                .percent(ram_percent)
                                .label(format!("{}%", ram_percent));

        //history
        let cpu_sparkline = Sparkline::default()
                                        .block(Block::bordered().title(" CPU History "))
                                        .data(&self.cpu_history)
                                        .style(Style::default().fg(ACCENT));

        frame.render_widget(cpu_gauge, cpu);
        frame.render_widget(ram_gauge, memory);
        frame.render_widget(cpu_sparkline, history);
        frame.render_widget(Paragraph::new(format!("Operating System : {}", self.os_name)), os);
    }

    fn handle_input(&mut self) {
        match self.input_buffer.as_str() {
            "nexus::home-open" => self.mode = Menu::MainMenu,
            "nexus::sysmot-open" => self.mode = Menu::SystemMonitor,
            _ => {}
        }
    }

}

pub fn panel(title: &str) -> Block<'static> {
    Block::bordered()
    .border_style(MUTED)
    .border_type(BorderType::Rounded)
    .title(format!(" {title} ").fg(ACCENT).bold())
}