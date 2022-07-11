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
    text::Span,
    widgets::{
        canvas::{Canvas, Rectangle},
        Block, Borders, Gauge, Sparkline,
    },
    Frame, Terminal,
};

use rand::{
    distributions::{Distribution, Uniform},
    rngs::ThreadRng, Rng,
};

use soloud::*;

#[derive(Clone)]
pub struct RandomSignal {
    distribution: Uniform<u64>,
    rng: ThreadRng,
}

impl RandomSignal {
    pub fn new(lower: u64, upper: u64) -> RandomSignal {
        RandomSignal {
            distribution: Uniform::new(lower, upper),
            rng: rand::thread_rng(),
        }
    }
}

impl Iterator for RandomSignal {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        Some(self.distribution.sample(&mut self.rng))
    }
}

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

    bump: u16,
    bump_tick: u64,

    signal: RandomSignal,
    streamdata: Vec<u64>,

    win: bool,
    win_time: f64,

    pongsound: Audio,
}

impl App {
    fn new() -> App {
        let mut signal = RandomSignal::new(0,100);
        let streamdata = signal.by_ref().take(200).collect::<Vec<u64>>();

        let pongsound= Audio::new(0);
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

            bump: 0,
            bump_tick: 0,

            signal,
            streamdata,

            win: false,
            win_time: 0.0,

            pongsound,
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
                if !self.win{
                    play_wav(&self.pongsound);
                }
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

        self.bump = ((self.bump_tick as f64 / 512.0) * 100.0) as u16;

        self.tick_count += 1;
        self.bump_tick += 1;

        if self.tick_count & 0x1FF == 0 { //bump the speed every 512 game ticks
            self.vx += 0.2;
            self.vy += 0.1;
            self.bump_tick = 0;
        }

        if self.win {
            if self.tick_count & 0xF == 0xF{
                let value = self.signal.next().unwrap();
            self.streamdata.pop();
            self.streamdata.insert(0, value);
            }   
        }
    }
}

struct Audio {
    sl: Soloud,
    wav: Wav,
}

impl Audio {
    fn new(select: u32) -> Audio {
        let sl = Soloud::default().unwrap();
        let mut wav = audio::Wav::default();
        match select {
            0 => wav.load_mem(include_bytes!("pong.wav")).unwrap(),
            1 => wav.load_mem(include_bytes!("victory.wav")).unwrap(),
            _ => panic!("Unable to access file")
        }
        Audio {
            sl, 
            wav,
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

        if app.score >= 10 {
            if app.win == false{
                app.win_time = (app.tick_count as f64 * 40.0) / 1000.0;
            }
            app.win = true;
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)].as_ref())
        .split(f.size());

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(chunks[1]);

    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL).title("Pong"))
        .paint(|ctx| {
            ctx.draw(&app.ball);
            ctx.draw(&app.board);
            
        })
        .x_bounds([10.0, 160.0])
        .y_bounds([10.0, 110.0]);
    f.render_widget(canvas, chunks[0]);

    if !app.win {
        let label = format!("{}/10", app.score);
        let gauge = Gauge::default()
            .block(Block::default().title("Score").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::White).bg(Color::Red))
            .percent(app.score * 10)
            .label(label);
        f.render_widget(gauge, bottom_chunks[0]);

        let label = format!("{}%", app.bump);
        let gauge = Gauge::default()
            .block(Block::default().title(format!("Level {}", ((app.vx - 0.8) / 0.2 + 1.0) as u8)).borders(Borders::LEFT | Borders::RIGHT))
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(app.bump)
            .label(label);
        f.render_widget(gauge, bottom_chunks[1]);

    }else{
        if app.tick_count & 0x20 == 0x20{
            let sparkline = Sparkline::default()
                .block(
                    Block::default()
                    .title("You Win!")
                    .borders(Borders::ALL)
                )
                .data(&app.streamdata)
                .style(Style::default().fg(Color::LightYellow));
            f.render_widget(sparkline, bottom_chunks[0]);
        } else {
            let sparkline = Sparkline::default()
                .block(
                    Block::default()
                    .title("You Win!")
                    .borders(Borders::ALL)
                )
                .data(&app.streamdata)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(sparkline, bottom_chunks[0]);
        }

        let canvas = Canvas::default()
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT).title("Timer"))
            .paint(|ctx| {
                ctx.print(
                    5.0, 25.0,
                    Span::styled(format!("{}", app.win_time), Style::default().fg(Color::Yellow)),
                );
            })
            .x_bounds([0.0, 50.0])
            .y_bounds([0.0, 50.0]);
        f.render_widget(canvas, bottom_chunks[1]);
    }
}

fn play_wav(file: &Audio){
    file.sl.play(&file.wav);
    /*while file.sl.voice_count() > 0 {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }*/
}