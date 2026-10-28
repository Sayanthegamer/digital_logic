package rust.logic_simulator;

import android.app.NativeActivity;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.Window;
import android.view.WindowManager.LayoutParams;

/**
 * NativeActivity host for the eframe / winit Android backend.
 * Replaces the legacy Macroquad QuadSurface + QuadNative JNI bridge.
 */
public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("logic_simulator");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        super.onCreate(savedInstanceState);
        applyImmersiveFullscreen();
    }

    @Override
    protected void onResume() {
        super.onResume();
        applyImmersiveFullscreen();
    }

    @SuppressWarnings("deprecation")
    private void applyImmersiveFullscreen() {
        Window window = getWindow();
        if (window == null) {
            return;
        }
        window.setFlags(LayoutParams.FLAG_LAYOUT_NO_LIMITS, LayoutParams.FLAG_LAYOUT_NO_LIMITS);
        if (Build.VERSION.SDK_INT >= 28) {
            window.getAttributes().layoutInDisplayCutoutMode =
                LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
        }
        if (Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(false);
        } else {
            View decorView = window.getDecorView();
            if (decorView != null) {
                int uiOptions = View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                    | View.SYSTEM_UI_FLAG_FULLSCREEN
                    | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
                decorView.setSystemUiVisibility(uiOptions);
            }
        }
    }
}
