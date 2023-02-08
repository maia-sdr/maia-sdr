#!/usr/bin/env python3

import argparse
import asyncio
import threading

import numpy as np
import matplotlib.pyplot as plt
import websockets


async def spectrum_loop(address, line):
    async with websockets.connect(address) as ws:
        while True:
            spec = np.frombuffer(await ws.recv(), 'float32')
            line.set_ydata(10*np.log10(spec))


def main_async(args, line):
    asyncio.run(spectrum_loop(args.ws_address, line))


def prepare_plot():
    plt.ion()
    fig = plt.figure()
    ax = fig.add_subplot(111)
    freqs = np.arange(4096)
    line, = ax.plot(freqs, np.zeros(freqs.size))
    ax.set_ylim((40, 100))
    return fig, ax, line


def parse_args():
    parser = argparse.ArgumentParser(
        description='Spectrum plot client for Maia SDR')
    parser.add_argument('ws_address', type=str,
                        help='websocket server address')
    return parser.parse_args()


def main():
    args = parse_args()
    fig, ax, line = prepare_plot()
    loop = threading.Thread(target=main_async, args=(args, line))
    loop.start()
    plt.show(block=True)


if __name__ == '__main__':
    main()
