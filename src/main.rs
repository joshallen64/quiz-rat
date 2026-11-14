use core::num;
use std::{
    env, io,
    str::FromStr,
    time::{Duration, Instant},
};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Wrap},
};
use ratcn::{
    Button, ButtonSize, Dialog, Input, InputState, List, ListItem, Theme, Toast, ToasterState,
    ToasterWidget,
    runtime::{Component, EventResult, FocusState, ModalState, Ratcn},
    terminal::{Session, SessionEvent, SessionOptions, termina},
};
use serde_json::Number;

mod exam;
use crate::{
    Msg::PrevQuestion,
    exam::{Answer, Exam, Question, QuestionBank},
};

/// Everything the app knows. `update` is the only place it changes.
struct AppState {
    focus: FocusState,
    toasts: ToasterState<'static>,
    modals: ModalState,
    exam: Exam,
    num_questions: InputState,
    current_answer: Option<Answer>,
    num_answered: Vec<usize>,
    num_correct: usize,
}

#[derive(Clone)]
enum Msg {
    FocusChanged(FocusState),
    ModalChanged(ModalState),
    QuestionChanged(usize),
    ExamLoaded(Exam),
    NumOfQuestions(InputState),
    QuestionAnswered(Answer),
    NextQuestion,
    PrevQuestion,
    Cancel,
    Start,
}

impl AppState {
    fn update(&mut self, msg: Msg, now: Duration) {
        match msg {
            Msg::FocusChanged(focus) => self.focus = focus,
            Msg::ModalChanged(modal) => self.modals = modal,
            Msg::Cancel => {
                self.modals.close(&mut self.focus).expect("close modal");
            }
            Msg::Start => {
                // self.toasts
                //     .push(Toast::loading("Generating Questions..."), now);
                self.exam.generate_exam();
                //
                // self.toasts.push(Toast::info("Quiz Started!!!"), now);
                self.modals.close(&mut self.focus).expect("close settings");

                self.update(Msg::QuestionChanged(0), now);
            }
            Msg::NumOfQuestions(input) => {
                self.num_questions = input.clone();
                self.exam.set_max_questions(input.value());
            }
            Msg::QuestionChanged(question_num) => {
                self.exam.current_question = Some(question_num);
                if let Some(questions) = &self.exam.exam_questions {
                    //set the answer for the question that has been saved in memory
                    let qnum = self
                        .exam
                        .current_question
                        .expect("No question number available");
                    // let _questions = &questions.expect("Couldn't get questions");
                    let question = questions.get(qnum).to_owned();
                    self.current_answer = question
                        .expect("couldn't get the current_question")
                        .user_answer
                        .to_owned();
                }
                if !self.modals.is_open("question") {
                    self.modals
                        .open("question", &mut self.focus)
                        .expect("open question");
                }
            }
            Msg::QuestionAnswered(answer) => {
                self.current_answer = Some(answer);
                let current_question = self
                    .exam
                    .current_question
                    .expect("How did you answer a question you didn't select???");
                if let Some(ans) = &self.current_answer {
                    if !self.num_answered.contains(&current_question) {
                        self.num_answered.push(current_question);
                    }
                    if ans.correct {
                        self.num_correct += 1;
                    }
                }
                if let Some(questions) = &mut self.exam.exam_questions {
                    let question = &mut questions.get_mut(current_question);
                    question
                        .as_mut()
                        .expect("no question found I guess")
                        .user_answer = self.current_answer.clone()
                }
            }
            Msg::NextQuestion => {
                if let Some(question) = self.exam.current_question {
                    if question < self.exam.max_questions {
                        let next = question + 1;
                        self.update(Msg::QuestionChanged(next), now);
                    }
                }
            }
            Msg::PrevQuestion => {
                if let Some(question) = self.exam.current_question {
                    if question > 0 {
                        let next = question - 1;
                        self.update(Msg::QuestionChanged(next), now);
                    }
                }
            }

            Msg::ExamLoaded(exam) => {}
        }
    }
}

struct App {
    state: AppState,
    ratcn: Ratcn<AppState, Msg>,
}

impl App {
    fn new(file_path: Option<String>) -> Self {
        let mut app_state = App {
            state: AppState {
                focus: FocusState::default(),
                toasts: ToasterState::default(),
                modals: ModalState::default(),
                exam: Exam::new(),
                num_questions: InputState::default(),
                current_answer: None,
                num_answered: Vec::new(),
                num_correct: 0,
            },
            ratcn: Ratcn::new().focus(|state: &AppState| &state.focus, Msg::FocusChanged),
        };
        app_state
            .state
            .modals
            .open("settings", &mut app_state.state.focus)
            .expect("settings_dialog");

        if let Some(filepath) = file_path {
            app_state
                .state
                .exam
                .load_file(filepath.as_str())
                .load_questions();
        }
        app_state.state.num_questions =
            InputState::new(app_state.state.exam.max_questions.to_string());

        app_state
    }

    /// Route one event; apply whatever message it produced.
    fn handle_event(&mut self, event: termina::Event, now: Duration) {
        if let EventResult::Emit(msg) = self.ratcn.handle_event(event, &self.state) {
            self.state.update(msg, now);
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame, theme: &Theme, now: Duration) {
        // Ratcn never reads a clock; the app says what time it is.
        self.state.toasts.prune_expired(now);

        let area = frame.area();
        let title_block = Block::bordered().title("QuizRat");
        frame.render_widget(title_block, area);

        self.ratcn.render(frame, area, &self.state, theme, |ctx| {
            if self.state.modals.is_open("settings") {
                ctx.modal("settings", settings_dialog(ctx.state()), ctx.area());
            }
            if self.state.modals.is_open("question") {
                let height = area.inner(Margin::new(0, 5)).height;
                ctx.modal(
                    "question",
                    question_display(ctx.state(), height),
                    ctx.area(),
                )
            }
        });
        frame.render_widget(
            ToasterWidget::new(&self.state.toasts, now).themed(theme),
            area,
        );
    }
}

fn question_display(state: &AppState, height: u16) -> Dialog<AppState, Msg> {
    let exam = &state.exam;
    let question_num = exam.current_question.expect("question_num") + 1;

    let maxq = state.exam.max_questions;
    let answered = state.num_answered.iter().count();
    let anscorrect = state.num_correct;

    Dialog::new()
        .outer_width(80)
        .title(format!(
            "Question {question_num} of {maxq} Answered {anscorrect} of {answered} correctly"
        ))
        .content(height, |ctx| {
            let layout = Layout::vertical([Constraint::Fill(30), Constraint::Fill(70)]).spacing(1);
            let [question_area, answer_area] = layout.areas(ctx.area());
            let cstate: &AppState = ctx.state();
            let current_num = cstate.exam.current_question.expect("no current question");
            let mut _question = {
                if let Some(questions) = &cstate.exam.exam_questions {
                    questions
                        .get(current_num)
                        .expect(
                            format!("Couldn't get questions from state \n {:#?}\n ", cstate.exam)
                                .as_str(),
                        )
                        .to_owned()
                } else {
                    Question::default()
                }
            };

            let answers = _question.answers;

            ctx.paint_widget(
                // Block::new().title(format!("{}", _question.question_text)),
                Paragraph::new(_question.question_text).wrap(Wrap { trim: true }),
                question_area,
            );
            ctx.component(
                "answersList",
                List::new(
                    answers
                        .into_iter()
                        .map(|answer| ListItem::new(answer.clone(), answer.answer_text)),
                )
                .selection(
                    |ctx: &AppState| {
                        let Some(answer) = &ctx.current_answer else {
                            return None;
                        };
                        Some(answer.clone())
                    },
                    Msg::QuestionAnswered,
                ),
                answer_area,
            );
        })
        .action(
            "prev",
            Button::new("Previous").on_press(|| Msg::PrevQuestion),
        )
        .action("next", Button::new("Next").on_press(|| Msg::NextQuestion))
}

fn settings_dialog(mut state: &AppState) -> Dialog<AppState, Msg> {
    Dialog::new()
        .title("Settings")
        .description("Stuff")
        .on_dismiss(|| Msg::Cancel)
        .content(10, |ctx| {
            let layout =
                Layout::vertical([Constraint::Max(1), Constraint::Fill(1), Constraint::Fill(1)]);
            let cstate: &AppState = ctx.state();
            let num_total_questions = cstate.exam.total_bank_questions;

            let [title, duration, number_of_questions] = layout.areas(ctx.area());
            ctx.paint_widget(
                Block::new().title(format!(
                    "Number of questions available: {num_total_questions}"
                )),
                title,
            );
            ctx.component("Duration", Input::new().title("Exam Duration"), duration);
            ctx.component(
                "Number of Questions",
                Input::new()
                    .title("Number of Questions")
                    .value(|ctx: &AppState| &ctx.num_questions, Msg::NumOfQuestions),
                number_of_questions,
            );
        })
        .action(
            "start",
            Button::new("Start Quiz")
                .destructive()
                .on_press(|| Msg::Start),
        )
}

fn main() -> io::Result<()> {
    let started = Instant::now();
    let mut my_args = Vec::from_iter(env::args());

    let file_path: Option<String> = if my_args.len() > 1 {
        Some(my_args.remove(1))
    } else {
        None
    };
    let mut app = App::new(file_path);

    let mut session = Session::open(SessionOptions::new().mouse().adaptive())?;

    loop {
        let now = started.elapsed();
        let theme = session.theme();
        session
            .terminal_mut()
            .draw(|frame| app.draw(frame, &theme, now))?;
        // A hand over the button, in terminals that can show one.
        session.set_pointer_shape(app.ratcn.pointer_shape())?;

        // Wait for input, or wake when the next toast is due to disappear.
        let timeout = app.state.toasts.time_until_next_expiry(now);
        if let Some(SessionEvent::Input(event)) = session.next(timeout)? {
            let quit = is_quit(&event);
            app.handle_event(event, started.elapsed());
            // A text field copies its selection on Ctrl+C; only a Ctrl+C that
            // copied nothing quits.
            match app.ratcn.take_clipboard() {
                Some(text) => session.set_clipboard(&text)?,
                None if quit => return Ok(()),
                None => {}
            }
        }
    }
}

fn is_quit(event: &termina::Event) -> bool {
    use termina::event::{KeyCode, KeyEventKind, Modifiers};

    matches!(
        event,
        termina::Event::Key(key)
            if key.kind == KeyEventKind::Press
                && key.code == KeyCode::Char('c')
                && key.modifiers.contains(Modifiers::CONTROL)
    )
}
