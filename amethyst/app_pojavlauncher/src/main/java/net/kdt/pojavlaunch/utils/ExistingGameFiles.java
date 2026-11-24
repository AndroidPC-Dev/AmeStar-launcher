package net.kdt.pojavlaunch.utils;

import android.os.Environment;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.List;

/**
 * Finds a Minecraft game folder (.minecraft layout: versions, libraries, assets) that another launcher
 * on the phone already filled with a version, and copies what AmeStar's own folder is missing, so only
 * the rest has to come from Mojang's servers. Only shared storage can be read: Android keeps other
 * apps' private folders out of reach.
 */
public final class ExistingGameFiles {
    public interface Progress {
        void onProgress(String what, int copied);
    }

    private ExistingGameFiles() {}

    /** Folders that hold `versions/<version>/<version>.jar`, other than `own`, most likely first. */
    public static List<File> find(String version, File own) {
        File storage = Environment.getExternalStorageDirectory();
        List<File> found = new ArrayList<>();
        List<File> bases = new ArrayList<>();
        File games = new File(storage, "games");
        File[] launchers = games.listFiles();
        if (launchers != null) for (File l : launchers) if (l.isDirectory()) bases.add(l);
        bases.add(storage);
        File[] top = storage.listFiles();
        if (top != null) for (File l : top) {
            if (l.isDirectory() && !l.getName().startsWith("Android") && !l.equals(own.getParentFile())) bases.add(l);
        }
        for (File base : bases) {
            for (File candidate : new File[]{new File(base, ".minecraft"), base, new File(base, "minecraft")}) {
                if (candidate.equals(own) || found.contains(candidate)) continue;
                if (new File(candidate, "versions/" + version + "/" + version + ".jar").isFile()) found.add(candidate);
            }
        }
        return found;
    }

    /** Copies what `dest` lacks of the version, its libraries and its assets. Returns the number of files copied. */
    public static int copy(File source, File dest, String version, Progress progress) throws IOException {
        int[] count = {0};
        copyTree(new File(source, "versions/" + version), new File(dest, "versions/" + version), count, progress);
        copyTree(new File(source, "libraries"), new File(dest, "libraries"), count, progress);
        copyTree(new File(source, "assets"), new File(dest, "assets"), count, progress);
        return count[0];
    }

    private static void copyTree(File from, File to, int[] count, Progress progress) throws IOException {
        File[] children = from.listFiles();
        if (children == null) return;
        if (!to.isDirectory() && !to.mkdirs()) throw new IOException("cannot create " + to);
        for (File child : children) {
            File target = new File(to, child.getName());
            if (child.isDirectory()) {
                copyTree(child, target, count, progress);
            } else if (!target.isFile() || target.length() != child.length()) {
                Files.copy(child.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING);
                count[0]++;
                if (progress != null && count[0] % 200 == 0) progress.onProgress(child.getName(), count[0]);
            }
        }
    }
}
