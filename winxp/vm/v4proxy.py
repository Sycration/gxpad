#!/usr/bin/env python3
"""An HTTP CONNECT proxy on 127.0.0.1 that opens its connections over IPv4 only.

On a network where IPv6 connections to Microsoft's download servers hang, xwin
(see XWIN_DIR at the top of build.sh) downloads through it:

  python3 winxp/vm/v4proxy.py &
  xwin --accept-license --https-proxy http://127.0.0.1:8899 --arch x86,x86_64 splat --output ~/.xwin

Usage: v4proxy.py [PORT]   (8899 by default)
"""
import asyncio
import socket
import sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8899


async def pipe(r, w):
    try:
        while data := await r.read(65536):
            w.write(data)
            await w.drain()
    except Exception:
        pass
    finally:
        w.close()


async def handle(cr, cw):
    try:
        head = await cr.readuntil(b"\r\n\r\n")
        method, target, _ = head.split(b"\r\n", 1)[0].decode().split(" ", 2)
        if method != "CONNECT":
            cw.write(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")
            cw.close()
            return
        host, port = target.rsplit(":", 1)
        ur, uw = await asyncio.open_connection(host, int(port), family=socket.AF_INET)
        cw.write(b"HTTP/1.1 200 Connection established\r\n\r\n")
        await cw.drain()
        await asyncio.gather(pipe(cr, uw), pipe(ur, cw))
    except Exception:
        cw.close()


async def main():
    server = await asyncio.start_server(handle, "127.0.0.1", PORT)
    async with server:
        await server.serve_forever()


asyncio.run(main())
