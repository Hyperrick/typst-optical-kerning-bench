on run argv
	if (count of argv) is 0 then error "Pass the JSX path as the first argument."
	set jsxPath to item 1 of argv
	set scriptArgs to {}
	if (count of argv) is greater than 1 then set scriptArgs to items 2 thru -1 of argv
	set jsxFile to POSIX file jsxPath as alias
	tell application id "com.adobe.InDesign"
		activate
		do script jsxFile language javascript with arguments scriptArgs
	end tell
end run
