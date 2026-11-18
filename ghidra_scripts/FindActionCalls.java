// Find functions that call the same callee with immediate first-arg 4 and 5 (or 6 and 7). Args: outFile
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.address.*;
import ghidra.program.model.scalar.Scalar;
import java.io.*;
import java.util.*;

public class FindActionCalls extends GhidraScript {
    @Override
    public void run() throws Exception {
        PrintWriter out = new PrintWriter(new FileWriter(getScriptArgs()[0]));
        Map<String, Map<String, Set<Long>>> m = new TreeMap<>(); // func -> callee -> args
        InstructionIterator it = currentProgram.getListing().getInstructions(true);
        Long lastPush = null; int since = 99;
        while (it.hasNext() && !monitor.isCancelled()) {
            Instruction ins = it.next();
            String mn = ins.getMnemonicString();
            if (mn.equals("PUSH")) {
                Object[] o = ins.getOpObjects(0);
                if (o.length == 1 && o[0] instanceof Scalar) { lastPush = ((Scalar) o[0]).getValue(); since = 0; continue; }
                lastPush = null; continue;
            }
            since++;
            if (mn.equals("CALL")) {
                if (lastPush != null && since <= 3 && lastPush >= 4 && lastPush <= 7) {
                    Function f = getFunctionContaining(ins.getAddress());
                    String fn = f == null ? ("nofunc@" + ins.getAddress()) : (f.getName() + "@" + f.getEntryPoint());
                    String callee = ins.getDefaultOperandRepresentation(0);
                    m.computeIfAbsent(fn, k -> new TreeMap<>()).computeIfAbsent(callee, k -> new TreeSet<>()).add(lastPush);
                }
                lastPush = null;
            }
        }
        for (Map.Entry<String, Map<String, Set<Long>>> e : m.entrySet())
            for (Map.Entry<String, Set<Long>> c : e.getValue().entrySet()) {
                Set<Long> s = c.getValue();
                if ((s.contains(4L) && s.contains(5L)) || (s.contains(6L) && s.contains(7L)))
                    out.println(e.getKey() + "\t" + c.getKey() + "\t" + s);
            }
        out.close();
    }
}
