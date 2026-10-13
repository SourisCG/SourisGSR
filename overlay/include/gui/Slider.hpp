#pragma once

#include "Widget.hpp"
#include <functional>
#include <string>

#include <mglpp/graphics/Color.hpp>
#include <mglpp/graphics/Text.hpp>
#include <mglpp/system/Clock.hpp>

namespace gsr {
    // SourisGSR: horizontal 0-100 style slider (used for per-input recording-only
    // volumes). Drag the knob, use Left/Right arrows while hovering, or
    // double-click to reset to the default value.
    class Slider : public Widget {
    public:
        Slider(const char *font_desc, int min_val, int max_val, int default_val, float bar_width);
        Slider(const Slider&) = delete;
        Slider& operator=(const Slider&) = delete;

        bool on_event(mgl::Event &event, mgl::Window &window, mgl::vec2f offset) override;
        void draw(mgl::Window &window, mgl::vec2f offset) override;

        mgl::vec2f get_size() override;

        int get_value() const;
        void set_value(int v);
        int get_default_value() const;

        std::function<void(int value)> on_changed;
    private:
        void set_value_from_x(float x);
        mgl::vec2f bar_size() const;
    private:
        int min_val = 0;
        int max_val = 100;
        int value = 100;
        int default_val = 100;
        float bar_width = 200.0f;
        std::string font_desc;
        mgl::Text value_text;
        bool dragging = false;
        mgl::Clock dbl_clock;
    };
}
