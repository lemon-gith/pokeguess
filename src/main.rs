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
    Settings
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
        let outline = "shadow-lg outline outline-black/5 dark:bg-slate-800 dark:shadow-none dark:-outline-offset-1 dark:outline-white/10";

        html! {
            <div class={"bg-blue-700 grid h-screen place-items-center absolute inset-0 size-32 box-border ".to_owned() + outline}>
                <div style="height:75%" class={"bg-green-700 flex flex-col items-center justify-center gap-y-5 size-64 box-border h-3/4 w-3/4 p-6 rounded-xl ".to_owned() + outline}>
                    <button class={"bg-white p-6 rounded-xl ".to_owned() + outline}
                        onclick={ctx.link().callback(|_| Msg::AddOne)}
                    >
                        { "+1" }
                    </button>
                    <p class={"bg-white p-6 max-w-sm rounded-xl ".to_owned() + outline}>
                        { self.value }
                    </p>
                </div>
            </div>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
