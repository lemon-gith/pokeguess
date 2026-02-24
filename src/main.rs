use yew::prelude::*;


enum Msg {
    AddOne,
}

struct App {
    value: i64,
    // TODO: put rustemon client in here
}

enum MenuOptions {
    StartGame,
    GameOptions,  // game-specific options, e.g. gens, difficulty, etc.
    Settings  // overall app settings
}

struct StartMenu {
    selection: Option<MenuOptions>,
}

impl Component for StartMenu {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self { selection: None }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <></>
        }
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self { value: 0 }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddOne => {
                self.value += 1;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let outline = "shadow-lg outline outline-black/5 dark:shadow-none dark:-outline-offset-1 dark:outline-white/10";

        html! {
            <div class={"bg-gray-500 flex flex-col items-center justify-center gap-y-5 size-64 box-border md:w-3/4 w-5/6 md:h-3/4 h-11/12 p-6 rounded-xl ".to_owned() + outline}>
                <button class={"bg-white p-2 rounded-xl cursor-pointer ".to_owned() + outline}
                    onclick={ctx.link().callback(|_| Msg::AddOne)}
                >
                    <img
                        src="https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/132.png"
                        alt="ditto" style="width: 100px"
                    />
                    <figcaption>
                        {"Click Me"}
                    </figcaption>
                </button>
                <p class={"bg-white p-6 max-w-sm rounded-xl ".to_owned() + outline}>
                    { self.value }
                </p>
            </div>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
