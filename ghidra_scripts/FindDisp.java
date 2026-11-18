// Find instructions whose operand text contains "+ 0xOFF]" for given offsets. Args: outFile off1 off2 ...
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import java.io.*;
import java.util.*;

public class FindDisp extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] a = getScriptArgs();
        PrintWriter out = new PrintWriter(new FileWriter(a[0]));
        List<String> pats = new ArrayList<>();
        for (int i = 1; i < a.length; i++) pats.add("+ 0x" + Long.toHexString(Long.decode(a[i])) + "]");
        InstructionIterator it = currentProgram.getListing().getInstructions(true);
        while (it.hasNext() && !monitor.isCancelled()) {
            Instruction ins = it.next();
            String s = ins.toString();
            for (String p : pats) {
                if (s.contains(p)) {
                    Function f = getFunctionContaining(ins.getAddress());
                    out.println(ins.getAddress() + "\t" + s + "\t" + (f == null ? "-" : f.getName() + "@" + f.getEntryPoint()));
                    break;
                }
            }
        }
        out.close();
    }
}
