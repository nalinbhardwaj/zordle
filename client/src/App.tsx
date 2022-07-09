import React, { useState, useEffect } from 'react';
import logo from './logo.svg';
import './App.css';
import init, { initThreadPool, get_play_diff } from 'halowordle';

function App() {
  const [ans, setAns] = useState(0);
  useEffect(() => {
    test();
  }, [])

  async function test() {
    console.log('here');
    await init();
    await initThreadPool(navigator.hardwareConcurrency);
    const play_diff = await get_play_diff("fluff", ["audio", "audio", "audio", "audio", "audio", "audio"]);
    setAns(play_diff);
  }

  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/App.tsx</code> and save to reload.
        </p>
        {ans}
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
