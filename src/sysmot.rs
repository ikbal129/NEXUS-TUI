use std::{collections::VecDeque, num::NonZeroU64};

use ratatui::{
    Frame, layout::{Constraint, Layout, Rect}, style::{Style, Stylize, Color}, text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Gauge, LineGauge, Sparkline}
};
use sysinfo::Cpu;

use crate::{ACCENT, App, HISTORY_LEN, MUTED, TITLE};
use crate::panel;

impl App {
    pub fn render_sysmot(&mut self, frame: &mut Frame) {
        let core_rows = self.system.cpus().len().div_ceil(2) as u16;
        let top_height = (core_rows + 3).max(5) + 2;

        let [header, top, processes, history, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(top_height),
            Constraint::Length(30),
            Constraint::Length(40),
            Constraint::Length(3)
        ]).areas(frame.area());

        let [cpu, memory] = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50)
        ]).areas(top);

        // self.sysmot_header(frame, header);
        self.header_panel(frame, header, "System Monitoring");
        self.render_cpu(frame, cpu);
        self.render_memory(frame, memory);
        frame.render_widget(panel("Processes"), processes);
        frame.render_widget(panel("History"), history);
        self.footer_panel(frame, footer);
    }

    fn render_cpu(&mut self, frame: &mut Frame, area: Rect) {
        let average = self.system.global_cpu_usage();
        let block = panel(&format!("CPU {average:.0}"));
        
        let core_rows = self.system.cpus().len().div_ceil(2) as u16;
        let inner = block.inner(area);

        let [cores_area, spark_area] = Layout::vertical([
            Constraint::Length(core_rows),
            Constraint::Fill(1)
        ]).spacing(2).areas(inner);

        let [left, rigth] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1)
        ]).spacing(2).areas(cores_area);

        for (i, cpu) in self.system.cpus().iter().enumerate() {
            let usage = cpu.usage() as f64;
            let column = if i % 2 == 0 {left} else {rigth};
            let row = Rect{
                y: column.y + (i/2) as u16,
                height: 1,
                ..column
            };

            let gauge = LineGauge::default()
                .ratio((usage / 100.0).clamp(0.0, 1.0))
                .label(format!("{usage:>3.0}%"))
                .filled_symbol("#")
                .unfilled_symbol(" ")
                .filled_style(level_color(usage))
                .unfilled_style(MUTED);

            frame.render_widget(gauge, row);
        }

        let width = spark_area.width as usize;
        let skip = self.cpu_history.len().saturating_sub(width);

        let data: Vec<u64> = self.cpu_history
            .iter()
            .skip(skip)
            .map(|v| *v as u64)
            .collect();

        let sparkline = Sparkline::default().data(&data).max(100).style(ACCENT);
        frame.render_widget(sparkline, spark_area);
        }

    fn render_memory(&mut self, frame: &mut Frame, area: Rect) {
        let gb = |bytes: u64| bytes as f64 / 1024.0 / 1024.0 / 1024.0;
        let sys = &self.system;

        let lines = vec![
            Line::from(format!(
                "RAM {:.1}/{:.1} GB",
                gb(sys.used_memory()),
                gb(sys.total_memory())
            )),
            Line::from(format!(
                "Swap {:.1}/{:.1} GB",
                gb(sys.used_swap()),
                gb(sys.total_swap())
            ))
        ];

        frame.render_widget(Paragraph::new(lines).block(panel("Memory")), area);
    }
}

pub fn level_color(percent: f64) -> Color {
    if percent < 50.0 {
        Color::Green
    } else if percent < 80.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

pub fn percent(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        used as f64 / total as f64 * 100.0
    }
}

pub fn used_gauge(name: &str, used: u64, total: u64) -> Gauge<'static> {
    let pct = percent(used, total);
    let gb = |bytes: u64| bytes as f64 / 1024.0 / 1024.0 / 1024.0;

    Gauge::default()
        .ratio(pct / 100.0)
        .label(format!(
            "{name} {:.1} / {:.1} GB ({pct:.0})",
            gb(used),
            gb(total)
        ))
        .gauge_style(Style::new().fg(level_color(pct)).bg(Color::Black))
        .use_unicode(true)
}

pub fn push_history(history: &mut VecDeque<f64>, value: f64) {
    history.push_back(value);

    if history.len() > HISTORY_LEN {
        history.pop_front();
    }
}