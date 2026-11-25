package net.kdt.pojavlaunch.fragments;

import static net.kdt.pojavlaunch.Tools.dialogOnUiThread;
import static net.kdt.pojavlaunch.Tools.hasMods;
import static net.kdt.pojavlaunch.Tools.hasNoOnlineProfileDialog;
import static net.kdt.pojavlaunch.Tools.hasOnlineProfile;
import static net.kdt.pojavlaunch.Tools.openPath;
import static net.kdt.pojavlaunch.Tools.runOnUiThread;
import static net.kdt.pojavlaunch.Tools.shareLog;

import android.app.Dialog;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.os.Bundle;
import android.view.View;
import android.widget.Button;
import android.widget.ImageButton;
import android.widget.Toast;

import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;
import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.appcompat.app.AlertDialog;
import androidx.fragment.app.Fragment;

import com.kdt.mcgui.mcVersionSpinner;

import net.kdt.pojavlaunch.CustomControlsActivity;
import net.kdt.pojavlaunch.R;
import net.kdt.pojavlaunch.Tools;
import net.kdt.pojavlaunch.extra.ExtraConstants;
import net.kdt.pojavlaunch.extra.ExtraCore;
import net.kdt.pojavlaunch.modloaders.LWJGL3ifyUtils;
import net.kdt.pojavlaunch.prefs.LauncherPreferences;
import net.kdt.pojavlaunch.utils.ExistingGameFiles;
import net.kdt.pojavlaunch.progresskeeper.ProgressKeeper;
import net.kdt.pojavlaunch.value.launcherprofiles.LauncherProfiles;
import net.kdt.pojavlaunch.value.launcherprofiles.MinecraftProfile;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.List;

public class MainMenuFragment extends Fragment {
    public static final String TAG = "MainMenuFragment";

    /** The one version the Starling engine plays. */
    private static final String STARLING_VERSION = "26.3";

    private mcVersionSpinner mVersionSpinner;

    private final ActivityResultLauncher<String[]> mContentPicker = registerForActivityResult(
            new ActivityResultContracts.OpenDocument(), this::installContent);

    public MainMenuFragment(){
        super(R.layout.fragment_launcher);
    }

    @Override
    public void onViewCreated(@NonNull View view, @Nullable Bundle savedInstanceState) {
        Button mCustomControlButton = view.findViewById(R.id.custom_control_button);
        Button mInstallJarButton = view.findViewById(R.id.install_jar_button);
        Button mShareLogsButton = view.findViewById(R.id.share_logs_button);
        Button mOpenDirectoryButton = view.findViewById(R.id.open_files_button);

        Button mPlayButton = view.findViewById(R.id.play_button);
        mVersionSpinner = view.findViewById(R.id.mc_version_spinner);

        mCustomControlButton.setOnClickListener(v -> startActivity(new Intent(requireContext(), CustomControlsActivity.class)));
        // AmeStar: Amethyst's "Execute a .jar" row installs Starling content instead
        mInstallJarButton.setText("Install .starling / .hmod file");
        mInstallJarButton.setOnClickListener(v -> mContentPicker.launch(new String[]{"*/*"}));

        mPlayButton.setOnClickListener(v -> startStarling());

        mShareLogsButton.setOnClickListener((v) -> shareLog(requireContext()));

        mOpenDirectoryButton.setOnClickListener((v)-> {
            if (Tools.isDemoProfile(v.getContext())){ // Say a different message when on demo profile since they might see the hidden demo folder
                hasNoOnlineProfileDialog(getActivity(), getString(R.string.demo_unsupported), getString(R.string.change_account));
            } else if (!hasOnlineProfile()) { // Otherwise display the generic pop-up to log in
                hasNoOnlineProfileDialog(requireActivity());
            } else openPath(v.getContext(), getCurrentProfileDirectory(), false);

        });


            Tools.swapFragment(requireActivity(), GamepadMapperFragment.class, GamepadMapperFragment.TAG, null);
            return true;
        });
    }

    private File getCurrentProfileDirectory() {
        String currentProfile = LauncherPreferences.DEFAULT_PREF.getString(LauncherPreferences.PREF_KEY_CURRENT_PROFILE, null);
        if(!Tools.isValidString(currentProfile)) return new File(Tools.DIR_GAME_NEW);
        LauncherProfiles.load();
        MinecraftProfile profileObject = LauncherProfiles.mainProfileJson.profiles.get(currentProfile);
        if(profileObject == null) return new File(Tools.DIR_GAME_NEW);
        return Tools.getGameDirPath(profileObject);
    }

    @Override
    public void onResume() {
        super.onResume();
        mVersionSpinner.reloadProfiles();
    }

    /**
     * AmeStar plays Minecraft 26.3 with the Starling engine. It is told where the game files this
     * launcher already holds are (so they are not downloaded again), and the engine starts in its
     * own process, straight into the game or, the first time, its setup.
     */
    private void startStarling() {
        final Context context = requireContext();
        try {
            Class.forName("io.github.juanluci.starling.GameActivity");
        } catch (ClassNotFoundException e) {
            Toast.makeText(context, "This build has no Starling engine in it", Toast.LENGTH_LONG).show();
            return;
        }
        final File minecraft = new File(Tools.DIR_GAME_NEW);
        // First take what another launcher on the phone (Amethyst, Pojav, Mojo...) already has of 26.3
        final AlertDialog looking = new AlertDialog.Builder(context)
                .setTitle("AmeStar")
                .setMessage("Looking for Minecraft 26.3 files in other launchers…")
                .setCancelable(false)
                .show();
        new Thread(() -> {
            String result = "";
            try {
                for (File source : ExistingGameFiles.find(STARLING_VERSION, minecraft)) {
                    final String name = source.getParentFile() == null ? source.getName() : source.getParentFile().getName();
                    runOnUiThread(() -> looking.setMessage("Copying 26.3 files from " + name + "…"));
                    int copied = ExistingGameFiles.copy(source, minecraft, STARLING_VERSION, (what, n) ->
                            runOnUiThread(() -> looking.setMessage("Copying files from " + name + ": " + n)));
                    result = "Reused " + copied + " files from " + name;
                    break;
                }
            } catch (IOException e) {
                result = "Could not copy from another launcher: " + e.getMessage();
            }
            final String message = result;
            runOnUiThread(() -> {
                looking.dismiss();
                if (!message.isEmpty()) Toast.makeText(context, message, Toast.LENGTH_LONG).show();
                launchStarling(context, minecraft);
            });
        }, "amestar-existing-files").start();
    }

    private void launchStarling(Context context, File minecraft) {
        File data = context.getFilesDir();
        try {
            if (!minecraft.isDirectory() && !minecraft.mkdirs()) throw new IOException("cannot create " + minecraft);
            writeText(new File(data, "amestar-minecraft-root"), minecraft.getAbsolutePath());
            // The engine starts the game, not its launcher, when this file is there
            writeText(new File(data, "launcher-play"), "");
        } catch (IOException e) {
            Toast.makeText(context, "Could not prepare Starling: " + e.getMessage(), Toast.LENGTH_LONG).show();
            return;
        }
        Intent game = new Intent();
        game.setClassName(context.getPackageName(), "io.github.juanluci.starling.GameActivity");
        game.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(game);
    }

    private void runOnUiThread(Runnable r) {
        android.app.Activity activity = getActivity();
        if (activity != null) activity.runOnUiThread(r);
    }

    private static void writeText(File file, String text) throws IOException {
        try (OutputStream out = new FileOutputStream(file)) {
            out.write(text.getBytes(java.nio.charset.StandardCharsets.UTF_8));
        }
    }

    /** Copies a picked .starling / .hmod / .zip file into the current profile's content folder. */
    private void installContent(@Nullable Uri uri) {
        if (uri == null) return;
        String name = "content";
        try (Cursor c = requireContext().getContentResolver().query(uri, null, null, null, null)) {
            if (c != null && c.moveToFirst()) {
                int index = c.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (index >= 0) name = c.getString(index);
            }
        }
        name = new File(name).getName();
        File dir = new File(getCurrentProfileDirectory(), "starling-content");
        File target = new File(dir, name);
        try (InputStream in = requireContext().getContentResolver().openInputStream(uri)) {
            if (in == null) throw new IOException("cannot open " + name);
            if (!dir.isDirectory() && !dir.mkdirs()) throw new IOException("cannot create " + dir);
            try (OutputStream out = new FileOutputStream(target)) {
                byte[] buffer = new byte[65536];
                int n;
                while ((n = in.read(buffer)) > 0) out.write(buffer, 0, n);
            }
            Toast.makeText(requireContext(), name + " added to " + dir.getName(), Toast.LENGTH_LONG).show();
        } catch (IOException e) {
            Toast.makeText(requireContext(), "Could not add " + name + ": " + e.getMessage(), Toast.LENGTH_LONG).show();
        }
    }

    private void runInstallerWithConfirmation(boolean isCustomArgs) {
        if (ProgressKeeper.getTaskCount() == 0)
            Tools.installMod(requireActivity(), isCustomArgs);
        else
            Toast.makeText(requireContext(), R.string.tasks_ongoing, Toast.LENGTH_LONG).show();
    }
}
