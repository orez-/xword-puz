import init, {CrosswordInput, generate_puz_file} from "./pkg/img2puz.js";

async function run() {
  await init("./pkg/img2puz_bg.wasm");
  document.getElementById("form")
    .addEventListener("submit", async event => {
      event.preventDefault();
      clearErrors();
      const formData = new FormData(event.target);
      const {
        across_clues, down_clues,
        title, author, copyright, notes,
        image,
      } = Object.fromEntries(formData);
      const imgBuf = await image.arrayBuffer();
      const image_array = new Uint8Array(imgBuf);
      let input = new CrosswordInput({
        across_clues, down_clues,
        title, author, copyright, notes,
        image: image_array,
      });
      let file_contents;
      try {
        file_contents = generate_puz_file(input);
      } catch (exc) {
        console.log(exc);
        if (exc instanceof Map) {
          for (let [key, value] of exc) {
            let node = document.getElementById(`${key}_error`);
            node.innerText = value;
            node.style.visibility = "visible";
          }
        } else {
          let node = document.getElementById("general_error");
          node.innerText = "Unexpected error";
          node.style.visibility = "visible";
        }
        return;
      }
      downloadBlob(file_contents, "out.puz", "application/octet-stream");
    });
  document.getElementById("imgUpload")
    .addEventListener("change", event => {
      const imgPreview = document.getElementById("gridPreview");
      const [file] = event.target.files;
      if (file) {
        imgPreview.src = URL.createObjectURL(file);
        imgPreview.onload = function() {
          URL.revokeObjectURL(imgPreview.src) // free memory
        }
      } else {
        imgPreview.src = "#";
      }
    });
  const modal = document.getElementById("modal");
  modal.addEventListener("click", event => {
    if (!document.getElementById("modal-inner").contains(event.target)) {
      closeModal();
    }
  });
  document.addEventListener("keyup", event => {
    if(event.key === "Escape") {
      closeModal();
    }
  });
}

const closeModal = () => {
  const modal = document.getElementById("modal");
  modal.classList.add("closed");
  window.setTimeout(() => { modal.style.visibility = "hidden"; }, 250);
}

window.openModal = () => {
  const modal = document.getElementById("modal");
  modal.classList.remove("closed");
  modal.style.visibility = "visible";
}

const downloadURL = (data, fileName) => {
  const a = document.createElement('a');
  a.href = data;
  a.download = fileName;
  document.body.appendChild(a);
  a.style.display = 'none';
  a.click();
  a.remove();
}

const downloadBlob = (data, fileName, mimeType) => {
  const blob = new Blob([data], {
    type: mimeType
  });
  const url = window.URL.createObjectURL(blob);
  downloadURL(url, fileName);
  setTimeout(() => window.URL.revokeObjectURL(url), 1000);
}

const clearErrors = () => {
  for (let elem of document.getElementsByClassName("error")) {
    elem.innerText = "";
    elem.style.visibility = "hidden";
  }
}

run();
