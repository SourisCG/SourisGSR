#include "../../include/gui/Slider.hpp"
#include "../../include/gui/Utils.hpp"
#include "../../include/Theme.hpp"
#include <mglpp/graphics/Rectangle.hpp>
#include <mglpp/window/Window.hpp>
#include <mglpp/window/Event.hpp>
#include <mglpp/system/FloatRect.hpp>
#include <mglpp/system/Clock.hpp>
#include <algorithm>
#include <cmath>

namespace gsr {
    Slider::Slider(const char *font_desc, int min_val, int max_val, int default_val, float bar_width) :
        min_val(min_val), max_val(max_val), value(default_val),
        default_val(default_val), bar_width(bar_width),
        font_desc(font_desc ? font_desc : ""), value_text("", font_desc)
    {
        if(this->min_val > this->max_val)
            std::swap(this->min_val, this->max_val);
        set_value(default_val);
    }

    int Slider::get_value() const {
        return value;
    }

    int Slider::get_default_value() const {
        return default_val;
    }

    void Slider::set_value(int v) {
        v = std::max(min_val, std::min(max_val, v));
        if(v != value) {
            value = v;
            if(on_changed)
                on_changed(value);
        }
    }

    mgl::vec2f Slider::bar_size() const {
        const float bar_height = std::max(4.0f, 0.008f * get_theme().window_height);
        return {bar_width, bar_height};
    }

    mgl::vec2f Slider::get_size() {
        if(!visible)
            return {0.0f, 0.0f};
        // Bar + gap + numeric readout ("100" wide).
        mgl::Text probe("100", font_desc.c_str());
        const mgl::vec2f text_size = probe.get_bounds().size;
        const float gap = 0.006f * get_theme().window_height;
        const mgl::vec2f bar = bar_size();
        return {bar.x + gap + text_size.x, std::max(bar.y, text_size.y)};
    }

    void Slider::set_value_from_x(float x) {
        const float frac = (x - position.x) / bar_width;
        const int range = max_val - min_val;
        set_value(min_val + (int)std::round(frac * range));
    }

    bool Slider::on_event(mgl::Event &event, mgl::Window &window, mgl::vec2f offset) {
        if(!visible)
            return true;

        const mgl::vec2f draw_pos = (position + offset).floor();
        const mgl::vec2f size = get_size().floor();
        const mgl::FloatRect hit(draw_pos, {size.x, std::max(size.y, bar_size().y + 8.0f)});

        if(event.type == mgl::Event::MouseButtonPressed && event.mouse_button.button == mgl::Mouse::Left) {
            const mgl::vec2f mouse_pos = {(float)event.mouse_button.x, (float)event.mouse_button.y};
            if(hit.contains(mouse_pos)) {
                const double since_press = dbl_clock.get_elapsed_time_seconds();
                dbl_clock.restart();
                if(since_press < 0.4) {
                    set_value(default_val); // double-click resets
                } else {
                    set_value_from_x(mouse_pos.x - offset.x);
                }
                dragging = true;
                set_widget_as_selected_in_parent();
                return false;
            }
            return true;
        }

        if(event.type == mgl::Event::MouseButtonReleased && event.mouse_button.button == mgl::Mouse::Left) {
            if(dragging) {
                dragging = false;
                remove_widget_as_selected_in_parent();
                return false;
            }
            return true;
        }

        if(event.type == mgl::Event::MouseMoved && dragging) {
            set_value_from_x((float)event.mouse_move.x - offset.x);
            return false;
        }

        if(event.type == mgl::Event::MouseWheelScrolled && dragging == false) {
            const mgl::vec2f mouse_pos = window.get_mouse_position().to_vec2f();
            if(hit.contains(mouse_pos) && !has_parent_with_selected_child_widget()) {
                set_value(value + (event.mouse_wheel_scroll.delta > 0 ? 5 : -5));
                return false;
            }
            return true;
        }

        if(event.type == mgl::Event::KeyPressed) {
            const mgl::vec2f mouse_pos = window.get_mouse_position().to_vec2f();
            if(!hit.contains(mouse_pos) || has_parent_with_selected_child_widget())
                return true;
            switch(event.key.code) {
                case mgl::Keyboard::Left:
                case mgl::Keyboard::Down:
                    set_value(value - 1);
                    return false;
                case mgl::Keyboard::Right:
                case mgl::Keyboard::Up:
                    set_value(value + 1);
                    return false;
                case mgl::Keyboard::Home:
                    set_value(min_val);
                    return false;
                case mgl::Keyboard::End:
                    set_value(max_val);
                    return false;
                default:
                    break;
            }
        }

        return true;
    }

    void Slider::draw(mgl::Window &window, mgl::vec2f offset) {
        if(!visible)
            return;

        const mgl::vec2f draw_pos = (position + offset).floor();
        const mgl::vec2f size = get_size().floor();
        const mgl::vec2f bar = bar_size();
        const float bar_y = draw_pos.y + (size.y - bar.y) * 0.5f;

        const bool hover = mgl::FloatRect(draw_pos, size).contains(window.get_mouse_position().to_vec2f()) && !has_parent_with_selected_child_widget();

        // Track.
        mgl::Rectangle track(bar);
        track.set_position({draw_pos.x, bar_y});
        track.set_color(mgl::Color(0, 0, 0, 120));
        window.draw(track);

        // Fill.
        const float frac = (max_val > min_val) ? (float)(value - min_val) / (float)(max_val - min_val) : 0.0f;
        if(frac > 0.001f) {
            mgl::Rectangle fill({bar.x * frac, bar.y});
            fill.set_position({draw_pos.x, bar_y});
            fill.set_color(get_color_theme().tint_color);
            window.draw(fill);
        }

        // Knob.
        const float knob_w = std::max(6.0f, 0.010f * get_theme().window_height);
        const float knob_x = draw_pos.x + bar.x * frac - knob_w * 0.5f;
        mgl::Rectangle knob({knob_w, size.y});
        knob.set_position({knob_x, draw_pos.y});
        knob.set_color(dragging ? get_color_theme().tint_color : mgl::Color(255, 255, 255));
        window.draw(knob);

        // Numeric readout.
        value_text.set_string(std::to_string(value));
        const float gap = 0.006f * get_theme().window_height;
        value_text.set_position({(draw_pos.x + bar.x + gap), (draw_pos.y + size.y * 0.5f - value_text.get_bounds().size.y * 0.52f)});
        value_text.set_color(get_color_theme().text_color);
        window.draw(value_text);

        if(hover || dragging) {
            draw_rectangle_outline(window, {draw_pos.x, bar_y}, bar, get_color_theme().tint_color, 1.0f);
        }
    }
}
