package net.kdt.pojavlaunch.prefs.screens;

import android.content.SharedPreferences;
import android.os.Bundle;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.preference.ListPreference;
import androidx.preference.Preference;
import androidx.preference.PreferenceCategory;
import androidx.preference.PreferenceScreen;
import androidx.preference.SeekBarPreference;
import androidx.preference.SwitchPreferenceCompat;

import net.kdt.pojavlaunch.Tools;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Iterator;
import java.util.Map;

/**
 * One page of the Starling engine's settings, built from assets/starling_settings.json (the
 * definitions of the AmeStar design). Values live in the launcher's preferences under
 * "st_&lt;key&gt;" and are written to Starling's settings file in the game folder on every change,
 * where the engine reads them at start.
 */
public class StarlingSettingsFragment extends LauncherPreferenceFragment {
    public static final String PREF_PREFIX = "st_";
    public static final String SETTINGS_FILE = "starling-settings.json";
    private static final String PRESET_KEY = "preset";
    private JSONObject mRoot;
    private boolean mApplyingPreset;

    @Override
    public void onCreatePreferences(Bundle b, String str) {
        Bundle args = getArguments();
        String pageId = args == null ? null : args.getString("page");
        PreferenceScreen screen = getPreferenceManager().createPreferenceScreen(requireContext());
        setPreferenceScreen(screen);
        try {
            mRoot = readDefinitions();
            JSONArray pages = mRoot.getJSONArray("pages");
            for (int i = 0; i < pages.length(); i++) {
                JSONObject page = pages.getJSONObject(i);
                if (!page.getString("id").equals(pageId)) continue;
                screen.addPreference(new net.kdt.pojavlaunch.prefs.BackButtonPreference(requireContext()));
                JSONArray groups = page.getJSONArray("groups");
                for (int g = 0; g < groups.length(); g++) addGroup(screen, groups.getJSONObject(g));
            }
        } catch (JSONException | IOException e) {
            Toast.makeText(requireContext(), "Starling settings: " + e.getMessage(), Toast.LENGTH_LONG).show();
        }
    }

    private JSONObject readDefinitions() throws IOException, JSONException {
        try (InputStream is = requireContext().getAssets().open("starling_settings.json")) {
            return new JSONObject(Tools.read(is));
        }
    }

    private void addGroup(PreferenceScreen screen, JSONObject group) throws JSONException {
        PreferenceCategory category = new PreferenceCategory(requireContext());
        category.setTitle(group.getString("title"));
        screen.addPreference(category);
        JSONArray items = group.getJSONArray("items");
        for (int i = 0; i < items.length(); i++) {
            Preference p = createPreference(items.getJSONObject(i));
            if (p != null) category.addPreference(p);
        }
    }

    private Preference createPreference(JSONObject def) throws JSONException {
        String type = def.getString("type");
        String key = PREF_PREFIX + def.getString("key");
        String desc = def.optString("desc", null);
        Preference p;
        switch (type) {
            case "toggle": {
                SwitchPreferenceCompat sw = new SwitchPreferenceCompat(requireContext());
                sw.setDefaultValue(def.getBoolean("def"));
                p = sw;
                break;
            }
            case "slider": {
                SeekBarPreference sb = new SeekBarPreference(requireContext());
                sb.setMin(def.getInt("min"));
                sb.setMax(def.getInt("max"));
                sb.setSeekBarIncrement(def.optInt("step", 1));
                sb.setShowSeekBarValue(true);
                sb.setDefaultValue(def.getInt("def"));
                sb.setUpdatesContinuously(true);
                String unit = def.optString("unit", "").trim();
                desc = (desc == null ? "" : desc + " ") + (unit.isEmpty() ? "" : "(" + unit + ")");
                p = sb;
                break;
            }
            case "select": {
                ListPreference lp = new ListPreference(requireContext());
                JSONArray options = def.getJSONArray("options");
                CharSequence[] entries = new CharSequence[options.length()];
                for (int i = 0; i < entries.length; i++) entries[i] = options.getString(i);
                lp.setEntries(entries);
                lp.setEntryValues(entries);
                lp.setDefaultValue(def.getString("def"));
                lp.setDialogTitle(def.getString("title"));
                lp.setSummaryProvider(ListPreference.SimpleSummaryProvider.getInstance());
                p = lp;
                break;
            }
            default: {
                p = new Preference(requireContext());
                String action = def.optString("action", "");
                p.setOnPreferenceClickListener(pref -> {
                    runAction(action);
                    return true;
                });
                break;
            }
        }
        p.setKey(type.equals("action") ? null : key);
        p.setTitle(def.getString("title"));
        if (desc != null && !desc.isEmpty() && !type.equals("select")) p.setSummary(desc);
        p.setIconSpaceReserved(false);
        return p;
    }

    private void runAction(String action) {
        if (action.startsWith("toast:")) {
            Toast.makeText(requireContext(), action.substring(6), Toast.LENGTH_SHORT).show();
        } else if (action.equals("reset")) {
            resetAll();
        } else {
            Toast.makeText(requireContext(), "Not available yet", Toast.LENGTH_SHORT).show();
        }
    }

    private void resetAll() {
        SharedPreferences prefs = getPreferenceManager().getSharedPreferences();
        if (prefs == null) return;
        SharedPreferences.Editor editor = prefs.edit();
        for (String key : prefs.getAll().keySet()) if (key.startsWith(PREF_PREFIX)) editor.remove(key);
        editor.apply();
        writeSettingsFile(prefs);
        Toast.makeText(requireContext(), "Starling settings reset", Toast.LENGTH_SHORT).show();
        requireActivity().getSupportFragmentManager().popBackStack();
    }

    @Override
    public void onSharedPreferenceChanged(@NonNull SharedPreferences p, String key) {
        super.onSharedPreferenceChanged(p, key);
        if (key == null || !key.startsWith(PREF_PREFIX) || mRoot == null) return;
        if (!mApplyingPreset) applyPresetRules(p, key);
        writeSettingsFile(p);
    }

    /** Choosing a preset sets the settings it covers; changing any of them switches to Custom. */
    private void applyPresetRules(SharedPreferences p, String key) {
        JSONObject table = mRoot.optJSONObject("presetTable");
        if (table == null) return;
        String presetKey = PREF_PREFIX + PRESET_KEY;
        mApplyingPreset = true;
        try {
            if (key.equals(presetKey)) {
                JSONObject values = table.optJSONObject(p.getString(presetKey, "Balanced"));
                if (values == null) return;
                SharedPreferences.Editor editor = p.edit();
                for (Iterator<String> it = values.keys(); it.hasNext(); ) {
                    String k = it.next();
                    Object v = values.get(k);
                    if (v instanceof Boolean) editor.putBoolean(PREF_PREFIX + k, (Boolean) v);
                    else if (v instanceof Number) editor.putInt(PREF_PREFIX + k, ((Number) v).intValue());
                    else editor.putString(PREF_PREFIX + k, String.valueOf(v));
                }
                editor.apply();
                // The visible widgets read their stored values again
                setPreferenceScreen(null);
                onCreatePreferences(null, null);
            } else if (!p.getString(presetKey, "Balanced").equals("Custom")) {
                Iterator<String> names = table.keys();
                boolean covered = false;
                while (names.hasNext() && !covered) {
                    covered = table.getJSONObject(names.next()).has(key.substring(PREF_PREFIX.length()));
                }
                if (covered) p.edit().putString(presetKey, "Custom").apply();
            }
        } catch (JSONException ignored) {
        } finally {
            mApplyingPreset = false;
        }
    }

    /** Writes every st_ value, by its plain key, where the engine looks for them. */
    public static void writeSettingsFile(SharedPreferences prefs) {
        try {
            JSONObject out = new JSONObject();
            for (Map.Entry<String, ?> e : prefs.getAll().entrySet()) {
                if (e.getKey().startsWith(PREF_PREFIX)) out.put(e.getKey().substring(PREF_PREFIX.length()), e.getValue());
            }
            File file = new File(Tools.DIR_GAME_HOME, SETTINGS_FILE);
            File dir = file.getParentFile();
            if (dir != null) dir.mkdirs();
            try (FileOutputStream fos = new FileOutputStream(file)) {
                fos.write(out.toString(2).getBytes(StandardCharsets.UTF_8));
            }
        } catch (JSONException | IOException ignored) {
        }
    }
}
