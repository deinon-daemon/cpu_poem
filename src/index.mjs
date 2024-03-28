import { h, render } from 'https://unpkg.com/preact@latest?module';
import { useState } from 'https://unpkg.com/preact@latest/hooks/dist/hooks.module.js?module';
import htm from "https://unpkg.com/htm@latest/dist/htm.module.js?module";



const html = htm.bind(h);


function App(props) {
    const [paused, setPaused] = useState("Snapshot");
    const handleSnapshot = () => {
        props.onSnapshot();
        setPaused(paused === "Snapshot" ? "Resume" : "Snapshot");
    };

    return html`
    <div>
      ${props.cpus.map((cpu) => {
        return html`<div class="bar">
          <div class="bar-inner" style="width: ${cpu}%"></div>
          <label>${cpu.toFixed(2)}%</label>
        </div>
        <div class="words">
            <p>
                ${props.sentences[props.cpus.indexOf(cpu)]}
            </p>
        </div>`
        
    })}
        <button onclick=${handleSnapshot}>${paused}</button>
    </div>
    

  `;
}

let i = 0;
let snapshotData = null;
let url = new URL("/realtime/cpus", window.location.href);
url.protocol = url.protocol.replace("http", "ws");

let ws = new WebSocket(url.href);
function startWebSocket() {
    ws = new WebSocket(url.href);
    ws.onmessage = (ev) => {
        let json = JSON.parse(ev.data);
        if (!snapshotData) {
            render(html`<${App} cpus=${json.cpus} sentences=${json.sentences} onSnapshot=${takeSnapshot}></${App}>`, document.body);
        }
    };
}

function takeSnapshot() {
    snapshotData = {
        cpus: [...document.querySelectorAll(".bar-inner")].map((bar) => parseFloat(bar.style.width)),
        sentences: [...document.querySelectorAll(".words p")].map((p) => p.textContent),
    };
    ws.close();
    render(html`<${App} cpus=${snapshotData.cpus} sentences=${snapshotData.sentences} onSnapshot=${restoreWebSocket}></${App}>`, document.body);
}

function restoreWebSocket() {
    snapshotData = null;
    startWebSocket();
}

startWebSocket();