use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Sparkline},
    Frame,
};
use tui_logger::TuiLoggerWidget;

use super::app::{App, Bytes};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    if app.show_help {
        draw_help(f, app);
        return;
    }

    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(9),
                Constraint::Percentage(60),
                Constraint::Percentage(0),
            ]
            .as_ref(),
        )
        .split(f.size());

    draw_stats(f, app, chunks[0]);
    draw_graphs(f, app, chunks[1]);
    draw_logs(f, app, chunks[2]);
}

fn draw_stats<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Length(5),
                Constraint::Length(0),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(area);

    let block = Block::default().title("Stats").borders(Borders::ALL);
    f.render_widget(block, area);

    let message = format!(
        "Server: {}, Bots connected: {:6}, Bytes tx: {}, Bytes rx: {}, Packets tx: {:6}, Packets rx: {:6}",
        app.server, app.bots, app.bytes_tx, app.bytes_rx, app.packets_tx, app.packets_rx
    );
    let status = Paragraph::new(message).block(Block::default().title("Status:"));
    f.render_widget(status, chunks[0]);

    let tps = Sparkline::default()
        .block(Block::default().title("TPS:"))
        .data(&app.tps);
    f.render_widget(tps, chunks[1]);
}

fn draw_graphs<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(40), Constraint::Percentage(0)].as_ref())
        .direction(Direction::Horizontal)
        .split(area);

    draw_players(f, app, chunks[0]);
    draw_bandwidth(f, app, chunks[1]);
}

fn draw_players<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let time_max = app
        .bot_count_data
        .iter()
        .map(|(x, _)| *x as u64)
        .max()
        .unwrap_or(0);
    let players_max = app
        .bot_count_data
        .iter()
        .map(|(_, y)| *y as u64)
        .max()
        .unwrap_or(0);

    let players = Chart::new(vec![Dataset::default()
        .data(&app.bot_count_data)
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)])
    .block(Block::default().title("Players").borders(Borders::ALL))
    .x_axis(
        Axis::default()
            .title(Span::styled("Time", Style::default().fg(Color::Red)))
            .style(Style::default().fg(Color::White))
            .bounds([0.0, time_max as f64]),
    )
    .y_axis(
        Axis::default()
            .title(Span::styled("Players", Style::default().fg(Color::Red)))
            .style(Style::default().fg(Color::White))
            .bounds([0.0, players_max as f64])
            .labels(
                [
                    "0".to_owned(),
                    (players_max / 2).to_string(),
                    players_max.to_string(),
                ]
                .into_iter()
                .map(Span::from)
                .collect(),
            ),
    );
    f.render_widget(players, area);
}

fn draw_bandwidth<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let time_max = app
        .bandwidth_in_data
        .iter()
        .map(|(x, _)| *x as u64)
        .max()
        .unwrap_or(0);
    let bandwidth_in_max = app
        .bandwidth_in_data
        .iter()
        .map(|(_, y)| *y as u64)
        .max()
        .unwrap_or(0);
    let bandwidth_out_max = app
        .bandwidth_out_data
        .iter()
        .map(|(_, y)| *y as u64)
        .max()
        .unwrap_or(0);
    let bandwidth_max = u64::max(bandwidth_in_max, bandwidth_out_max);

    let bandwidth = Chart::new(vec![
        Dataset::default()
            .name("Client bound")
            .data(&app.bandwidth_in_data)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Red)),
        Dataset::default()
            .name("Serverbound")
            .data(&app.bandwidth_out_data)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Blue)),
    ])
    .block(Block::default().title("Bandwidth").borders(Borders::ALL))
    .x_axis(
        Axis::default()
            .title(Span::styled("Time", Style::default().fg(Color::Red)))
            .style(Style::default().fg(Color::White))
            .bounds([0.0, time_max as f64]),
    )
    .y_axis(
        Axis::default()
            .title(Span::styled("Bandwidth", Style::default().fg(Color::Red)))
            .style(Style::default().fg(Color::White))
            .bounds([0.0, bandwidth_max as f64])
            .labels(
                [
                    format!("{}/s", Bytes(0)),
                    format!("{}/s", Bytes(bandwidth_max / 2)),
                    format!("{}/s", Bytes(bandwidth_max)),
                ]
                .into_iter()
                .map(Span::from)
                .collect(),
            ),
    );
    f.render_widget(bandwidth, area);
}

fn draw_logs<B: Backend>(f: &mut Frame<B>, _app: &mut App, area: Rect) {
    let logs =
        TuiLoggerWidget::default().block(Block::default().title("Logs").borders(Borders::ALL));
    f.render_widget(logs, area);
}

fn draw_help<B: Backend>(f: &mut Frame<B>, _app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .margin(15)
        .split(f.size());

    let text = Paragraph::new(include_str!("help.txt"))
        .block(Block::default().title("Help").borders(Borders::ALL));
    f.render_widget(text, chunks[0]);
}
