const out = 1; const o = { out }; o.out; function f(out: string) { return out } type Box<out> = out;
