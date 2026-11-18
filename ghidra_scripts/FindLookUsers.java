// Functions that FLD both [reg+0x28] and [reg+0x2c] from the same base register and contain FMUL. Args: outFile
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import java.io.*;
import java.util.*;

public class FindLookUsers extends GhidraScript {
    @Override
    public void run() throws Exception {
        PrintWriter out = new PrintWriter(new FileWriter(getScriptArgs()[0]));
        FunctionIterator fi = currentProgram.getFunctionManager().getFunctions(true);
        while (fi.hasNext() && !monitor.isCancelled()) {
            Function f = fi.next();
            Map<String, Integer> mask = new HashMap<>();
            boolean fmul = false;
            for (Instruction ins : currentProgram.getListing().getInstructions(f.getBody(), true)) {
                String s = ins.toString();
                if (s.startsWith("FMUL")) fmul = true;
                if (s.startsWith("FLD float ptr [")) {
                    int p = s.indexOf('['); String inner = s.substring(p + 1, s.length() - 1);
                    String[] parts = inner.split(" \\+ ");
                    if (parts.length == 2) {
                        if (parts[1].equals("0x28")) mask.merge(parts[0], 1, (x, y) -> x | y);
                        if (parts[1].equals("0x2c")) mask.merge(parts[0], 2, (x, y) -> x | y);
                    }
                }
            }
            if (fmul) for (Map.Entry<String, Integer> e : mask.entrySet())
                if (e.getValue() == 3) { out.println(f.getEntryPoint() + "\t" + f.getName() + "\t" + e.getKey() + "\tsize=" + f.getBody().getNumAddresses()); break; }
        }
        out.close();
    }
}
