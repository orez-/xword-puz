import init, {hello} from "./pkg/img2puz.js";

async function run() {
  await init("./pkg/img2puz_bg.wasm");
  document.getElementById("submit")
    .addEventListener("click", event => {
      let x = hello();
      alert(x);
    });
}

run();
