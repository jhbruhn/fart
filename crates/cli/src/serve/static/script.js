const theme = document.getElementById("theme");

function updateTheme() {
  document.body.classList.remove("light");
  document.body.classList.remove("dark");
  const data = new FormData(theme);
  document.body.classList.add(data.get("theme"));
}

for (const input of theme.querySelectorAll("input")) {
  input.addEventListener("input", updateTheme);
}

updateTheme();

const userConstsForm = document.getElementById("user-consts");

function rerun() {
  const data = new FormData(userConstsForm);
  const consts = {};

  for (const [name, val] of data.entries()) {
    if (val !== "") {
      consts[name] = val;
    }
  }

  fetch("/rerun", {
    method: "POST",
    cache: "no-cache",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(consts)
  });
}

function like() {
  fetch("/like", {
    method: "POST",
    cache: "no-cache",
    headers: {
      "Content-Type": "application/json",
    }
  });
}


function debounce(f) {
  let id = null;
  return function debounced(...args) {
    clearTimeout(id);
    id = setTimeout(() => f.apply(this, args), 300);
  };
}

const debouncedRerun = debounce(rerun);

const regenerate = document.getElementById("regenerate");

regenerate.addEventListener("click", event => {
  event.preventDefault();
  rerun();
});

const like_button = document.getElementById("like");
like_button.addEventListener("click", event => {
    event.preventDefault();
    like();
});

class UserConst {
  constructor(name, ty, value) {
    this.name = name;
    this.ty = ty;
    this.value = value;
    this.used = true;
    this.element = document.createElement("div");
    this.label = document.createElement("label");
    this.input = document.createElement("input");
    this.onInput = this.onInput.bind(this);

    this.element.className = "hbox user-const";

    this.update(name, ty, value);
    this.element.appendChild(this.label);

    this.input.addEventListener("input", this.onInput);
    if((ty.includes("32") || ty.includes("64")) && !name.includes("RNG_SEED")) {
      this.input.setAttribute("type", "number");
      if(ty.includes("f32") || ty.includes("f64")) 
        this.input.setAttribute("step", "0.1");
    } else {
      this.input.setAttribute("type", "text");
    }

    if(name.includes("RNG_SEED")) {
        this.randomize = document.createElement("button");
        this.randomize.innerHTML =  "RANDOMIZE";
        this.randomize.addEventListener("click", event => {
            event.preventDefault();
            this.update(name, ty, Math.floor(Math.random() * 1000000000));
            debouncedRerun();
        });
    }

    this.element.appendChild(this.input);
    if(this.randomize)
      this.element.appendChild(this.randomize);
  }

  onInput(event) {
    event.preventDefault();
    debouncedRerun();
  }

  update(name, ty, value) {
    this.name = name;
    this.ty = ty;
    this.value = value;
    this.used = true;
    this.label.textContent = `${name}: ${ty} =`;
    this.input.setAttribute("name", name);
    this.input.setAttribute("value", value);
  }

  destroy() {
    this.input.removeEventListener("input", this.onInput);
  }
}

class UserConstSet {
  constructor(container) {
    this.container = container;
    this.consts = new Map;
  }

  insert(name, ty, value) {
    let c = this.consts.get(name);
    if (c == null) {
      c = new UserConst(name, ty, value);
      this.container.appendChild(c.element);
      this.consts.set(name, c);
    } else {
      c.update(name, ty, value);
    }
  }

  sweep() {
    const newConsts = new Map;

    for (const [name, c] of this.consts) {
      if (c.used) {
        c.used = false;
        newConsts.add(name, c);
      } else {
        this.container.removeChild(c.element);
        c.destroy();
      }
    }

    this.consts = newConsts;
  }
}

const logs = document.getElementById("logs");
const latest = document.querySelector("#latest > img");
const events = new EventSource("/events");
const userConsts = new UserConstSet(userConstsForm);

events.addEventListener("start", _ => logs.textContent = "");
events.addEventListener("output", e => {
  const data = JSON.parse(e.data);
    console.log(e.data);
  for (const [_, name, ty, value] of data.matchAll(/.*fart: const ([\w_]+): ([\w_]+) = (.+);.*/g)) {
    userConsts.insert(name, ty, value);
  }
  logs.textContent += data;
});
events.addEventListener("finish", _ => {
    setTimeout(function() {
        latest.src = `./images/latest.svg#${Date.now()}-${Math.random()}`;
    }, 5);
});
events.onerror = event => {
  logs.textContent = `Error: disconnected from ${window.location.host}/events.`;
  console.error(event);
  regenerate.setAttribute("disabled", "");
  like_button.setAttribute("disabled", "");
};
