// Resource and command identifiers of gxpad for Windows.
#ifndef GXPAD_RESOURCE_H
#define GXPAD_RESOURCE_H

#define IDI_GXPAD               1
#define IDB_TOOLBAR             2
#define IDR_MENU                100
#define IDR_ACCEL               101
#define IDR_CONTEXT             102
#define IDR_CONTEXT_PAPER       103
#define IDR_CONTEXT_LIST        104

#define IDD_DIMENSION           200
#define IDD_VARIABLE            201
#define IDD_FUNCTION            202
#define IDD_POINT               203
#define IDD_SHAPE               204
#define IDD_ABOUT               205
#define IDD_HELP                206

// controls of the dialogs
#define IDC_NAME                1001
#define IDC_VALUE               1002
#define IDC_DRIVING             1003
#define IDC_MEASURED            1004
#define IDC_NUMERIC             1005
#define IDC_SUPP                1006
#define IDC_KIND_TEXT           1007
#define IDC_FREE                1008
#define IDC_SOLVED              1009
#define IDC_DEFINED             1010
#define IDC_EXPR                1011
#define IDC_EQUATION            1012
#define IDC_EQUATION_NOTE       1013
#define IDC_X                   1014
#define IDC_Y                   1015
#define IDC_LOCKED              1016
#define IDC_CONSTRUCTION        1017
#define IDC_INFINITE            1018
#define IDC_VERSION             1019
#define IDC_LIBVERSION          1020
#define IDC_TOPICS              1021
#define IDC_TOPIC_TEXT          1022
#define IDC_ABOUT_ICON                1023
#define IDC_HINT                1024
#define IDC_LICENSE             1025

// child windows of the main window
#define IDC_REBAR               1100
#define IDC_TOOLBAR             1101
#define IDC_CONBAR              1102
#define IDC_TOOLBOX             1103
#define IDC_STATUS              1104
#define IDC_CANVAS              1105
#define IDC_PANE                1106
#define IDC_ZOOM                1107
#define IDC_TABS                1110
#define IDC_LIST                1111
#define IDC_PANE_ADD            1112
#define IDC_PANE_PROPS          1113
#define IDC_PANE_DELETE         1114
#define IDC_FORMULA_BOX         1115
#define IDC_FORMULA_TEXT        1116
#define IDC_FORMULA_VIEW        1117
#define IDC_FORMULA_COPY        1118
#define IDC_FORMULA_LATEX       1119
#define IDC_FORMULA_LONGER      1120
#define IDC_FORMULA_PROGRESS    1121
#define IDC_FORMULA_STATE       1122
#define IDC_DIM_EDIT            1123

// commands; the string with the same identifier is the help line of the
// status bar
#define ID_FILE_NEW             40001
#define ID_FILE_OPEN            40002
#define ID_FILE_SAVE            40003
#define ID_FILE_SAVE_AS         40004
#define ID_FILE_EXPORT          40005
#define ID_FILE_PRINT           40006
#define ID_FILE_EXIT            40007
#define ID_FILE_MRU1            40011
#define ID_FILE_MRU2            40012
#define ID_FILE_MRU3            40013
#define ID_FILE_MRU4            40014

#define ID_EDIT_UNDO            40021
#define ID_EDIT_REDO            40022
#define ID_EDIT_COPY_FORMULA    40023
#define ID_EDIT_COPY_LATEX      40024
#define ID_EDIT_COPY_PICTURE    40025
#define ID_EDIT_COPY_DRAWING    40026
#define ID_EDIT_DELETE          40027
#define ID_EDIT_SELECT_ALL      40028
#define ID_EDIT_PROPERTIES      40029

#define ID_VIEW_TOOLBAR         40041
#define ID_VIEW_CONBAR          40042
#define ID_VIEW_TOOLBOX         40043
#define ID_VIEW_STATUSBAR       40044
#define ID_VIEW_PANE            40045
#define ID_VIEW_ZOOM_IN         40046
#define ID_VIEW_ZOOM_OUT        40047
#define ID_VIEW_FIT             40048
#define ID_VIEW_GRID            40049
#define ID_VIEW_SYMBOLS         40050

#define ID_INSERT_FUNCTION      40061
#define ID_INSERT_VARIABLE      40062

// ID_TOOL_FIRST + each Tool, in the order of the Tool enumeration
#define ID_TOOL_FIRST           40100
#define ID_TOOL_SELECT          40100
#define ID_TOOL_POINT           40101
#define ID_TOOL_LINE            40102
#define ID_TOOL_CIRCLE          40103
#define ID_TOOL_CIRCLE3         40104
#define ID_TOOL_ARC             40105
#define ID_TOOL_ARC3            40106
#define ID_TOOL_ELLIPSE         40107
#define ID_TOOL_HYPERBOLA       40108
#define ID_TOOL_PARABOLA        40109
#define ID_TOOL_ELLIPSE_FOCI    40110
#define ID_TOOL_HYPERBOLA_FOCI  40111
#define ID_TOOL_PARABOLA_FOCUS  40112
#define ID_TOOL_DIMENSION       40113
#define ID_TOOL_MIDPOINT        40114
#define ID_TOOL_INTERSECT       40115
#define ID_TOOL_PERP_LINE       40116
#define ID_TOOL_PAR_LINE        40117
#define ID_TOOL_PERP_BISECTOR   40118
#define ID_TOOL_ANGLE_BISECTOR  40119
#define ID_TOOL_TANGENT_LINE    40120
#define ID_TOOL_REFLECT         40121
#define ID_TOOL_FOCI            40122
#define ID_TOOL_DIRECTRIX       40123
#define ID_TOOL_ASYMPTOTES      40124
#define ID_TOOL_LAST            40124

// ID_CON_FIRST + each GX_CON_* kind that can be added
#define ID_CON_FIRST            40200
#define ID_CON_COINCIDENT       40200
#define ID_CON_ON_CURVE         40201
#define ID_CON_MIDPOINT         40202
#define ID_CON_HORIZONTAL       40203
#define ID_CON_VERTICAL         40204
#define ID_CON_PARALLEL         40205
#define ID_CON_PERPENDICULAR    40206
#define ID_CON_COLLINEAR        40207
#define ID_CON_EQUAL            40208
#define ID_CON_CONCENTRIC       40209
#define ID_CON_TANGENT          40210
#define ID_CON_SYMMETRIC        40211
#define ID_CON_LAST             40211
#define ID_CON_LOCK             40220
#define ID_CON_CONSTRUCTION     40221
#define ID_CON_INFINITE         40222

// ID_DIM_FIRST + each GX_DIM_* kind
#define ID_DIM_FIRST            40300
#define ID_DIM_DISTANCE         40300
#define ID_DIM_HDIST            40301
#define ID_DIM_VDIST            40302
#define ID_DIM_POINT_LINE       40303
#define ID_DIM_LENGTH           40304
#define ID_DIM_RADIUS           40305
#define ID_DIM_DIAMETER         40306
#define ID_DIM_ANGLE            40307
#define ID_DIM_AXIS_A           40308
#define ID_DIM_AXIS_B           40309
#define ID_DIM_X                40310
#define ID_DIM_Y                40311
#define ID_DIM_LAST             40311
#define ID_DIM_DRIVING          40320
#define ID_DIM_NUMERIC          40321
#define ID_DIM_SUPP             40322
#define ID_FORMULA_LONGER       40323

#define ID_HELP_TOPICS          40401
#define ID_HELP_ABOUT           40402

// strings that are not help lines
#define IDS_APP                 100
#define IDS_UNTITLED            101
#define IDS_READY               102
#define IDS_FILTER              103
#define IDS_PNG_FILTER          104
#define IDS_SAVE_CHANGES        105
#define IDS_OPEN_FAILED         106
#define IDS_SAVE_FAILED         107
#define IDS_TOOL_PROMPT_FIRST   1200

#endif
