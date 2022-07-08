use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Rectangle},
        Block, Borders, Gauge,
    },
    text::Span,
    Frame, Terminal,
};

use rand::Rng;


struct App {
    ball: Rectangle,
    board: Rectangle,
    
    playground: Rect,
    vx: f64,
    vy: f64,
    dir_x: bool,
    dir_y: bool,

    score: u16,
    tick_count: u64,
}

impl App {
    fn new() -> App {
        App {
            ball: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 5.0,
                height: 5.0,
                color: Color::Red,
            },
            board: Rectangle {
                x: 10.0,
                y: 10.0,
                width: 10.0,
                height: 3.0,
                color: Color::White,
            },
            playground: Rect::new(10, 10, 150, 100),
            vx: 1.0,
            vy: 1.0,
            dir_x: true,
            dir_y: true,

            score: 0,
            tick_count: 0,
        }
    }

    fn on_tick(&mut self) {

        let ball_bounds = vec![
            self.ball.x - self.ball.width / 2.0,
            self.ball.x + self.ball.width / 2.0,
        ];
        let board_bounds = vec![
            self.board.x - self.board.width / 2.0,
            self.board.x + self.board.width / 2.0, 
        ];

        if self.ball.x < self.playground.left() as f64
            || self.ball.x + self.ball.width > self.playground.right() as f64
        {
            self.dir_x = !self.dir_x;
        }
        if self.ball.y < self.playground.top() as f64
            || self.ball.y + self.ball.height > self.playground.bottom() as f64
        {
            self.dir_y = !self.dir_y;
        }

        if ball_bounds[0] > board_bounds[0] && ball_bounds[0] < board_bounds[1]
            || ball_bounds[1] < board_bounds[1] && ball_bounds[1] > board_bounds[0]
        {
            if self.ball.y < 30.0{
                self.ball.color = Color::Yellow;
            }
            
            if self.ball.y < self.board.y + self.board.height
            {
                if !self.dir_y {self.score += 1;}
                self.dir_y = true;
            }
        } else {
            self.ball.color = Color::Red
        }

        if self.dir_x {
            self.ball.x += self.vx;
        } else {
            self.ball.x -= self.vx;
        }

        if self.dir_y {
            self.ball.y += self.vy;
        } else {
            self.ball.y -= self.vy
        }

        self.tick_count += 1;
        if self.tick_count & 0x3FF == 0 { //bump the speed every 1024 game ticks
            self.vx += 0.2;
            self.vy += 0.1;
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(25);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    {
        let mut rng = rand::thread_rng();
        app.ball.x = rng.gen_range(10.0..90.0);
        app.ball.y = rng.gen_range(10.0..100.0);
    }

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Right => {
                        if app.board.x + 10.0 < app.playground.right().into(){
                            app.board.x += 5.0;
                        }
                    }
                    KeyCode::Left => {
                        if app.board.x > app.playground.left().into(){
                            app.board.x -= 5.0;
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)].as_ref())
        .split(f.size());

    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL).title("Pong"))
        .paint(|ctx| {
            ctx.draw(&app.ball);
            ctx.draw(&app.board);
            
        })
        .x_bounds([10.0, 160.0])
        .y_bounds([10.0, 110.0]);
    f.render_widget(canvas, chunks[0]);

    if app.score < 10 {
        let label = format!("{}/10", app.score);
        let gauge = Gauge::default()
        .block(Block::default().title("Score").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::White).bg(Color::Red))
        .percent(app.score * 10)
        .label(label);

        f.render_widget(gauge, chunks[1]);
    }else{
        let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL).title("You Win!"))
        .paint(|ctx| {
            ctx.print(
                0.0,
                0.0,
                Span::styled(r"You Win!", Style::default().bg(Color::LightYellow).fg(Color::Black)),
            );
        })
        .x_bounds([-180.0, 180.0])
        .y_bounds([-90.0, 90.0]);
        f.render_widget(canvas, chunks[1]);

    }
}
