import java.io.*;
import java.lang.reflect.*;

/**
 * Dumps PlantUML's embedded Liberation Sans metric tables to a binary file.
 * Usage: java -cp <plantuml classes> DumpMetrics.java out.bin
 * Format: sections of [u32 LE length][length x u16 LE].
 * Sections: 4 fonts (REGULAR, BOLD, ITALIC, BOLD_ITALIC) x [header(5), ranges, advances];
 * then measured widths [ranges, widths]; then the width table [block count, per block: data].
 */
public class DumpMetrics {
	static DataOutputStream out;

	static void section(int[] v) throws IOException {
		out.write(le32(v.length));
		for (int x : v)
			out.write(new byte[] { (byte) x, (byte) (x >> 8) });
	}

	static byte[] le32(int x) {
		return new byte[] { (byte) x, (byte) (x >> 8), (byte) (x >> 16), (byte) (x >> 24) };
	}

	static int[] chars(String s) {
		int[] r = new int[s.length()];
		for (int i = 0; i < r.length; i++)
			r[i] = s.charAt(i);
		return r;
	}

	static Object field(Object o, String name) throws Exception {
		Field f = o.getClass().getDeclaredField(name);
		f.setAccessible(true);
		return f.get(o);
	}

	public static void main(String[] args) throws Exception {
		out = new DataOutputStream(new BufferedOutputStream(new FileOutputStream(args[0])));
		String pkg = "net.sourceforge.plantuml.klimt.drawing.font.";
		Class<?> lsm = Class.forName(pkg + "LiberationSansMetrics");
		for (String n : new String[] { "REGULAR", "BOLD", "ITALIC", "BOLD_ITALIC" }) {
			Field f = lsm.getDeclaredField(n);
			f.setAccessible(true);
			Object m = f.get(null);
			int[] head = new int[5];
			String[] names = { "unitsPerEm", "ascent", "descent", "lineGap", "missingAdvance" };
			for (int i = 0; i < 5; i++)
				head[i] = (Integer) field(m, names[i]);
			section(head);
			section(chars((String) field(m, "ranges")));
			section(chars((String) field(m, "advances")));
		}
		Field f = lsm.getDeclaredField("WINDOWS_SANS_SERIF");
		f.setAccessible(true);
		Object mw = f.get(null);
		section(chars((String) field(mw, "ranges")));
		section(chars((String) field(mw, "widths")));
		byte[][] table = (byte[][]) Class.forName(pkg + "UnicodeFontWidthSansSerif").getField("SANS_SERIF").get(null);
		section(new int[] { table.length });
		for (byte[] b : table) {
			int[] v = new int[b.length];
			for (int i = 0; i < v.length; i++)
				v[i] = b[i] & 0xFF;
			section(v);
		}
		out.close();
	}
}
