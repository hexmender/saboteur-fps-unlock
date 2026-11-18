// Decompile functions. Args: outFile item...
//   item = 0xADDR        decompile function containing ADDR
//   item = !0xADDR       delete the function that starts at ADDR (fake chunk/thunk function)
//   item = +0xADDR       (re)create a function at ADDR, then decompile it
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.io.*;

public class Decomp extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        PrintWriter out = new PrintWriter(new FileWriter(args[0]));
        DecompInterface di = new DecompInterface();
        di.openProgram(currentProgram);
        for (int i = 1; i < args.length; i++) {
            String s = args[i];
            if (s.startsWith("del_")) {
                Address a = toAddr(Long.decode(s.substring(4)));
                Function f = getFunctionAt(a);
                if (f != null) removeFunction(f);
                continue;
            }
            if (s.startsWith("flow_")) {
                Address entry = toAddr(Long.decode(s.substring(5)));
                AddressSet body = new AddressSet();
                java.util.ArrayDeque<Address> q = new java.util.ArrayDeque<>();
                q.add(entry);
                int guard = 0;
                while (!q.isEmpty() && guard++ < 200000) {
                    Address x = q.poll();
                    if (body.contains(x)) continue;
                    Instruction ins = getInstructionAt(x);
                    if (ins == null) { disassemble(x); ins = getInstructionAt(x); }
                    if (ins == null) { clearListing(x, x.add(15)); disassemble(x); ins = getInstructionAt(x); }
                    if (ins == null) continue;
                    body.add(ins.getMinAddress(), ins.getMaxAddress());
                    if (ins.getMnemonicString().equalsIgnoreCase("JMP")) { ins.setFlowOverride(ghidra.program.model.listing.FlowOverride.NONE); for (Address t : ins.getFlows()) q.add(t); Address ref = ins.getAddress(0); if (ref != null) q.add(ref); continue; }
                    if (ins.getFlowType().isCall()) { if (ins.getFallThrough() != null) q.add(ins.getFallThrough()); continue; }
                    if (ins.getFlowType().isTerminal()) continue;
                    if (ins.getFallThrough() != null) q.add(ins.getFallThrough());
                    for (Address t : ins.getFlows()) q.add(t);
                }
                for (Function g : currentProgram.getFunctionManager().getFunctions(body, true)) {
                    if (!g.getEntryPoint().equals(entry)) removeFunction(g);
                }
                Function f0 = getFunctionContaining(entry);
                if (f0 != null) removeFunction(f0);
                Function f = createFunction(entry, null);
                try { f.setBody(body); } catch (Exception e) { out.println("// setBody failed: " + e); }
                DecompileResults r = di.decompileFunction(f, 600, monitor);
                out.println("// ==== FLOW " + f.getName() + " @ " + entry + " body=" + body.getNumAddresses());
                out.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// failed: " + r.getErrorMessage());
                out.flush();
                continue;
            }
            boolean recreate = s.startsWith("new_");
            Address a = toAddr(Long.decode(recreate ? s.substring(4) : s));
            Function f = recreate ? getFunctionAt(a) : getFunctionContaining(a);
            if (recreate && f != null) { removeFunction(f); f = null; }
            if (f == null) {
                disassemble(a);
                f = createFunction(a, null);
            }
            if (f == null) { out.println("// no function at " + s); continue; }
            DecompileResults r = di.decompileFunction(f, 300, monitor);
            out.println("// ==== " + f.getName() + " @ " + f.getEntryPoint() + " (query " + s + ") body=" + f.getBody().getNumAddresses());
            out.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// failed: " + r.getErrorMessage());
            out.flush();
        }
        out.close();
    }
}



