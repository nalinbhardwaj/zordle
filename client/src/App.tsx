import React, { useState, useEffect } from 'react';
import logo from './logo.svg';
import './App.css';
import { wrap } from 'comlink';

function App() {
  const worker = new Worker(new URL('./test-worker', import.meta.url), {
    name: 'my-worker',
    type: 'module',
  });
  const workerApi = wrap<import('./test-worker').MyFirstWorker>(worker);
  const [ans, setAns] = useState(0);

  async function test() {
    const ret2 = await workerApi.get_play_diff();
    console.log('in between', ret2);
    // const params = await workerApi.write_params();
    // console.log('outside params', params);
    const proof = await workerApi.prove_play();
    console.log('outside proof', proof);
  }

  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/App.tsx</code> and save to reload.
        </p>
        {ans}
        <button onClick={test}>test</button>
        <a
          className="App-link"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
      </header>
    </div>
  );
}

export default App;
