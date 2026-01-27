Some modules support a `profiles` configuration option, which utilises the standardised configuration profiles system.
This allows a subset of the module's configuration options to be overwritten depending on a defined value.
The value is related to the module's state, and is normally a number of some sort. 
Examples include volume and brightness percentage.

Profiles are defined as a map of the profile name to its state matcher (the special `when` key) and configuration.
The state matches on any value **equal to or below** the specified threshold.

Properties supported by the profiles system are marked as such 
on a module's documentation page in the configuration section.

> [!NOTE]
> As the system is very new, only a small subset of options support this feature currently.
> This is expected to increase over time.

```corn
let {
  $volume = {
    // default - applies if no other profile is active
    icons.volume = "󰕾"
    
    profiles = {
        // medium profile activates when volume <= 67%
        medium.when = 67
        medium.icons.volume = "󰖀"

        // low profile activates when volume <= 33%
        low.when = 33
        low.icons.volume = "󰕿"

        // other properties supported by the profile can also be updated
        low.icons.muted = "icons:volume-low-muted" 
    }
  }
} in {
  end = [ $volume ]
}
```

## Compound State

Some modules additionally support 'compound' state, allowing for the profile to match on more than one value.
For example the battery module supports matching on both the charge percentage and whether the battery is currently charging.

More specific values for state matchers with the same value will take precedent.  

```corn
let {
  $battery = {
    type = "battery"
  
    format = "HIGH {percentage}%"
  
    profiles = {
        low.when = { percent = 20 }
        low.format = "LOW {percentage}%"
  
        // applies over `low` when charging.
        low-charging.when = { percent = 20 charging = true }
        low-charging.format = "LOW (CHARGING) {percentage}%"

        // applies over `medium-charging` when NOT charging.
        medium.when = { percent = 50 charging = false }
        medium.format = "MEDIUM {percentage}%"
  
        medium-charging.when = { percent = 50 }
        medium-charging.format = "MEDIUM (CHARGING) {percentage}%"
  
        good.when = { percent = 75 charging = false }
        good.format = "GOOD {percentage}%"
  
        good-charging.when = { percent = 75 charging = true }
        good-charging.format = "GOOD (CHARGING) {percentage}%"
  
        empty.when = { percent = 1 charging = true }
    } 
} in {
  end [ $battery ]
}
```

## Styling

When a profile is active, a class name is appended to the module's root widget in the form `.profile-{name}`.
This allows you to apply profile-specific styling as follows:

```css
.battery.profile-medium {
    color: yellow;
}

.battery.profile-low {
    color: red;
}
```

## Shorthand syntax

If you wish to define a profile for styling only, and do not wish to override any configuration this is possible too.

While perfectly possible to write the matcher with no configuration as below:

```corn
let {
  $volume = {
    profiles.low.when = 33
    profiles.medium.when = 67
  }
} in {
    end = [ $volume ] 
}
```

...Ironbar also supports omitting the `when` key and attaching the state matcher directly.

```corn
let {
  $volume = {
    profiles.low = 33
    profiles.medium = 67
  }
} in {
    end = [ $volume ] 
}
```